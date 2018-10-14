extern crate itertools;
extern crate regex;
extern crate serde_json;
extern crate yas_to_vscode;

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use yas_to_vscode::*;

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let from = args.next().unwrap();
    let dest = args.next().unwrap();
    let dest = Path::new(&dest);
    for d in fs::read_dir(Path::new(&from)).unwrap() {
        let d = d.unwrap();
        let path = d.path();
        let lang = path.file_name().unwrap();
        let lang = String::from_utf8(lang.as_bytes().to_vec()).unwrap();
        let lang = &lang[0..lang.len() - 5];
        if d.metadata().unwrap().is_dir() {
            let colls: HashMap<_, _> = fs::read_dir(&path)
                .unwrap()
                .into_iter()
                .flat_map(|file| -> Option<_> {
                    let file = file.ok()?;
                    let path = file.path();
                    let name = String::from_utf8(path.file_name()?.as_bytes().to_vec()).ok()?;
                    Snippet::parse(&name, &fs::read_to_string(&path).ok()?)
                        .map(|c| Some((name, c.result)))
                        .ok()?
                }).map(|(name, snip)| {
                    if snip
                        .body
                        .iter()
                        .any(|l| Regex::new(r"(^|[^\\])`\(|\$\$\(.+\)|\$\(").unwrap().is_match(l))
                    {
                        println!("NOTE: `{}' in {} might have some uninterpreted code", name, lang)
                    }
                    (name, snip)
                }).collect();
            {
                let mut path = dest.join(lang);
                path.set_extension("json");
                let mut buf = fs::File::create(path).unwrap();
                serde_json::ser::to_writer_pretty(buf, &colls).unwrap();
            }
        }
    }
}
