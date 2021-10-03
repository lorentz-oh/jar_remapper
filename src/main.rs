use std::fs;
use std::io::{self, prelude::*};
use std::collections::{hash_map, HashMap};
use zip::read::ZipFile;
use zip::result::ZipResult;
use regex::{Match, Regex};

struct Mappings{
    fields: hash_map::HashMap<String, String>,
    methods: hash_map::HashMap<String, String>,
    params: hash_map::HashMap<String, String>
}

impl Mappings{
	fn new(file: &fs::File) -> Option<Self>{
        let mut archive = match zip::ZipArchive::new(file){
            Ok(v) => {v}
            Err(_) => {return None;}
        };

        let mut fields  = (HashMap::<String, String>::new(), "fields.csv");
        let mut params  = (HashMap::<String, String>::new(), "params.csv");
        let mut methods = (HashMap::<String, String>::new(), "methods.csv");

        for entry in vec![&mut fields, &mut params, &mut methods].iter_mut(){
            if let Ok(mut file) = archive.by_name(entry.1){
                if let Err(()) = Self::fill_mapping(&mut entry.0, &mut file){
                    return None;
                }
            }
        }

        Some(Self{fields: fields.0, params: params.0, methods: methods.0})
	}

    fn fill_mapping(map: &mut HashMap<String, String>, file: &mut ZipFile) -> Result<(), ()>{
        let reader = io::BufReader::new(file);
        for line in reader.lines(){
            let line = line.unwrap_or("".to_string());
            let cols: Vec<&str> = line.split(",").collect();
            let obf = cols.get(0);
            let name = cols.get(1);
            if obf.is_some() && name.is_some(){
                map.insert(obf.unwrap().to_string(), name.unwrap().to_string());
            }else{
                return Err(());
            }
        }
        Ok(())
    }
}

struct JarRemapper{
    mappings: Mappings,
}

impl JarRemapper {
    fn new(mappings: Mappings) -> Self{
        Self{mappings}
    }

    fn remap_jar(&self, jar: fs::File) -> Result<(),String>{
        let mut archive = zip::ZipArchive::new(jar).expect("Couldn't read jar");
        for i in 0..archive.len(){
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
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
                self.remap_file(&mut file, &mut outfile);
            }
        }
        return Ok(());
    }

    fn remap_file(&self, file: &mut ZipFile, outfile: &mut fs::File){
        if let Some(name) = file.enclosed_name(){
            if !String::from(name.to_str().unwrap()).ends_with(".java"){
                return;
            }
        }
        let mut buf = String::new();
        file.read_to_string(&mut buf);
        Self::replace_regex_matches_from_map(&mut buf, r"field_\d+_[[:alpha:]]+", &self.mappings.fields);
        Self::replace_regex_matches_from_map(&mut buf, r"func_\d+_[[:alpha:]]+", &self.mappings.methods);
        Self::replace_regex_matches_from_map(&mut buf, r"p_\d+_\d+_", &self.mappings.params);
        outfile.write(buf.as_bytes());
    }

    fn replace_regex_matches_from_map(buf: &mut String, regex: &str, map: &HashMap<String, String>){
        let re = Regex::new(regex).expect("Incorrect regex");
        let mut out = String::new();
        let matches: Vec<_> = re.find_iter(buf.as_str()).collect();

        for (i, m) in matches.iter().enumerate(){
            let start_before = match matches.get(i-1) {
                None => {0}
                Some(prev_m) => {prev_m.end()+1}
            };
            let end_before = m.start() - 1;
            out.push_str(&buf.as_str()[start_before..end_before]);
            let name = match map.get(m.as_str()){
                None => {m.as_str()}
                Some(v) => {v.as_str()}
            };
            out.push_str(name);
        }
        if let Some(last_match) = matches.last(){
            out.push_str(&buf.as_str()[last_match.end()+1..]);
        }
        buf.clone_from(&out);
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <mappings> <jar>", args[0]);
        return;
    }
    let mappings = fs::File::open(args.get(1).unwrap()).expect("Couldn't open mappings file");
    let mut jar = fs::File::open(args.get(2).unwrap()).expect("Couldn't open jar");

    if let Some(mappings) = Mappings::new(&mappings){
        let jar_remapper = JarRemapper::new(mappings);
        if let Err(e) = jar_remapper.remap_jar(jar){
            println!("{}", e);
        }
    }
}
