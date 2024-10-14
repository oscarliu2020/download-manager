use anyhow::{bail, Context, Result};
use indicatif::ProgressBar;
use reqwest::header::HeaderName;
use reqwest::Url;
use std::{io::Cursor, pin::Pin};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    sync::Semaphore,
};
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3";
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum State {
    Idle,
    Downloading,
    Done,
    Failed,
}
#[derive(Debug)]
pub enum FileBackend {
    Memory(memmap2::MmapMut),
    File(tokio::fs::File),
}
pub struct Item {
    source: String,
    state: State,
    pub size: u64,
    backend: FileBackend,
    pub filename: String,
}
static SEM: Semaphore = Semaphore::const_new(50);
impl Item {
    pub async fn new(source: String) -> Result<Self> {
        let client = reqwest::Client::new();
        let response = client
            .head(&source)
            .header(
                HeaderName::from_lowercase(b"user-agent").unwrap(),
                USER_AGENT,
            )
            .send()
            .await
            .with_context(|| format!("[Request Error] Url: {source}"))?;
        let url = Url::parse(&source).context("[Url Parse]")?;
        let filename = url
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");
        // if headers
        if response.status().is_success() {
            let size = response
                .headers()
                .get("content-length")
                .with_context(|| format!("[Content-Length] Url: {source}"))?
                .to_str()
                .with_context(|| format!("[to_str] Url: {source}"))?
                .parse::<u64>()
                .with_context(|| format!("[to_u64] Url: {source}"))?;
            if size == 0 {
                bail!("[Size Is Zero] Url: {source}");
            }
            let backend: FileBackend = if size > (2 << 30) {
                // 2GB
                let fs = tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .read(true)
                    .open(filename)
                    .await
                    .with_context(|| "file creation error")?;
                fs.set_len(size).await?;
                let mmap = unsafe { memmap2::MmapMut::map_mut(&fs).with_context(|| "mmap error")? };
                FileBackend::Memory(mmap)
            } else {
                let fs = tokio::fs::File::create(filename)
                    .await
                    .with_context(|| "file creation error")?;
                FileBackend::File(fs)
            };
            Ok(Self {
                source,
                state: State::Idle,
                size,
                backend,
                filename: filename.to_string(),
            })
        } else {
            bail!("[Response] Url: {source} Status: {}", response.status());
        }
    }
    pub async fn download(&mut self, client: reqwest::Client, pg: ProgressBar) -> Result<()> {
        let mut writer: Pin<Box<dyn AsyncWrite + Send + Sync>> = match &mut self.backend {
            FileBackend::Memory(mmap) => Box::pin(Cursor::new(mmap.as_mut())),
            FileBackend::File(fs) => {
                let buf = BufWriter::new(fs);
                Box::pin(buf)
            }
        };
        let permit = SEM.acquire().await.unwrap();
        let mut response = client
            .get(&self.source)
            .send()
            .await
            .with_context(|| "[Response Error]")?;
        while let Some(chunk) = response.chunk().await? {
            pg.inc(chunk.len() as u64);
            writer.write_all(&chunk).await?;
        }
        writer.flush().await?;
        drop(writer);
        drop(permit);
        if let FileBackend::Memory(mmap) = &self.backend {
            mmap.flush().unwrap();
        }
        pg.finish();
        Ok(())
    }
}
impl Drop for Item {
    fn drop(&mut self) {
        if let FileBackend::Memory(mmap) = &mut self.backend {
            mmap.flush().unwrap();
        }
    }
}
