use std::{env, fs::{self, File}, io::{BufWriter, Write}};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 {
        let input = fs::read_to_string(&args[1]).unwrap();
        let writer = BufWriter::new(File::create(&args[2]).unwrap());

        let mut deserializer = ron::Deserializer::from_str(&input).unwrap();
        let mut serializer = serde_yaml::Serializer::new(writer);
        serde_transcode::transcode(&mut deserializer, &mut serializer).unwrap();
        serializer.into_inner().flush().unwrap();
    } else {
        println!("No 2 arguments");
    }
}
