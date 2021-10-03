use std::fs;
use std::io::{self, prelude::*};
use std::collections::{hash_map, HashMap};
use zip::read::ZipFile;
use zip::result::ZipResult;

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
        let mut archive = zip::ZipArchive::new(jar);
        todo!();
        return Ok(());
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
