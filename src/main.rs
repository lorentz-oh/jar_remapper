use std::fs;
use std::io::{self, prelude::*};
use std::collections::HashMap;
use std::path::Path;
use zip::read::ZipFile;
use regex::Regex;

fn get_mapping(file: &fs::File) -> Result<HashMap<String, String>,()>{
    let mut archive = match zip::ZipArchive::new(file){
        Ok(v) => {v}
        Err(_) => {return Err(());}
    };

    let mut mapping = HashMap::<String, String>::new();

    for entry in vec!["fields.csv", "params.csv", "methods.csv"].iter_mut(){
        if let Ok(mut file) = archive.by_name(entry){
            if let Err(()) = fill_mapping(&mut mapping, &mut file){
                return Err(());
            }
        }
    }

    Ok(mapping)
}

fn fill_mapping(map: &mut HashMap<String, String>, file: &mut ZipFile) -> Result<(), ()>{
    let reader = io::BufReader::new(file);
    for (i, line) in reader.lines().enumerate(){
        if i == 0{
            continue;}
        let line = line.unwrap_or("".to_string());
        let cols: Vec<&str> = line.split(",").collect();
        let obf = cols.get(0);
        let name = cols.get(1);
        if obf.is_some() && name.is_some(){
            map.insert(obf.unwrap().to_string(), name.unwrap().to_string());
        }
    }
    Ok(())
}

struct JarRemapper{
    mappings: HashMap<String, String>,
}

impl JarRemapper {
    fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }

    fn remap_jar(&self, jar_name: &String) -> Result<(), String> {
        let jar = match fs::File::open(jar_name) {
            Ok(v) => {v}
            Err(_) => {return Err(String::from("Couldn't open jar"))}
        };
        let mut archive = match zip::ZipArchive::new(jar){
            Ok(v) => {v}
            Err(_) => {return Err(String::from("Couldn't read jar"))}
        };
        let re = Regex::new(r"(field|func|p)_i*\d+_[a-zA-Z0-9]+_?").expect("Incorrect regex");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let outpath = match jar_name.find(".jar") {
                None => { outpath }
                Some(v) => { Path::new(&jar_name.as_str()[..v]).join(outpath) }
            };

            if (&*file.name()).ends_with('/') {
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p).unwrap();
                    }
                }
                let mut outfile = fs::File::create(&outpath).unwrap();
                self.remap_file(&mut file, &mut outfile, &re);
            }
        }
        return Ok(());
    }

    fn remap_file(&self, file: &mut ZipFile, outfile: &mut fs::File, re: &Regex) {
        let name = match file.enclosed_name(){
            Some(v) => {String::from(v.to_str().unwrap_or(""))}
            None => {String::new()}
        };

        if !name.ends_with(".java") {
            io::copy(file, outfile);
            return;
        }

        let mut buf = String::new();
        file.read_to_string(&mut buf);
        let matches: Vec<_> = re.find_iter(buf.as_str()).collect();

        if matches.len() == 0{
            outfile.write(buf.as_bytes());
            return;
        }

        for (i, m) in matches.iter().enumerate() {
            let start_before = if i > 0 {
                matches.get(i - 1).unwrap().end()
            } else { 0 };
            let end_before = m.start();
            outfile.write(&buf.as_bytes()[start_before..end_before]);
            let name = match self.mappings.get(m.as_str()) {
                None => { m.as_str() }
                Some(v) => { v.as_str() }
            };
            outfile.write(name.as_bytes());
        }

        if let Some(last_match) = matches.last() {
            outfile.write(&buf.as_bytes()[last_match.end()..]);
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <mappings> <jar>", args[0]);
        return;
    }
    let mappings = fs::File::open(args.get(1).unwrap()).expect("Couldn't open mappings file");
    let jar = args.get(2).unwrap();

    if let Ok(mappings) = get_mapping(&mappings){
        let jar_remapper = JarRemapper::new(mappings);
        if let Err(e) = jar_remapper.remap_jar(jar){
            println!("{}", e);
        }
    }
}
