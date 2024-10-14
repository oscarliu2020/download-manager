use clap::Parser;
/// read url from argument or file and download it
#[derive(Debug,Parser)]
#[command(version,about,long_about=None)]
pub struct Cmd{
    #[arg(short,long)]
    url:Vec<String>,
    #[arg(short,long)]
    file:Option<std::path::PathBuf>,
    #[arg(short,long)]
    pub verbose:bool,
    #[arg(short,long)]
    pub output_dir:Option<std::path::PathBuf>,
}
impl Cmd {
    #[must_use]
    pub fn get_targets(&self)->Vec<String>{
        use std::fs::File;
        use std::io::{BufRead,BufReader};
        let mut res=vec![];
        if let Some(file)=&self.file {
            let file=File::open(file).expect("file not found");
            let mut reader=BufReader::new(file);
            let mut buf=String::new();
            while reader.read_line(&mut buf).unwrap()>0 {
                let url=buf.trim();
                res.push(url.to_owned());
                buf.clear();
            }
        }
        res.extend_from_slice(&self.url);
        res
    }
}