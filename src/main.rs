#[macro_use] extern crate structopt;
#[macro_use] extern crate failure_derive;
#[macro_use] extern crate hyper;

extern crate serde_yaml;
extern crate serde_json; 
extern crate walkdir;
extern crate failure;
extern crate url;
extern crate reqwest;

use std::path::{PathBuf, StripPrefixError};
use structopt::StructOpt;
use walkdir::WalkDir;
use url::{Url, ParseError};
use hyper::header::{Headers, ContentType};
use std::fs;
use std::io;
use serde_yaml::Value;
use std::ffi::OsStr;


#[derive(StructOpt, Debug)]
#[structopt(name = "vault thing")]
struct Opt {
    #[structopt(short = "d", long = "directory", default_value = ".", parse(from_os_str))]
    directory: PathBuf,

    #[structopt(long = "vault-uri", env = "VAULT_URI")]
    vault_uri: Url,

    #[structopt(long = "vault-token", env = "VAULT_TOKEN")]
    vault_token: String
}

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "We have some kind of error: {}", _0)]
    DirReadError(#[cause] walkdir::Error),

    #[fail(display = "We have some kind of error: {}", _0)]
    StripPrefix(#[cause] StripPrefixError),

    #[fail(display = "We have some kind of error: {}", _0)]
    UriJoinError(#[cause] ParseError),

	#[fail(display = "We have some kind of error: {}", _0)]
    VaultUpdate(#[cause] reqwest::Error),

    #[fail(display = "We have some kind of error")]
    NoParent,

   #[fail(display = "We have some kind of error")]
    PathToStr,

   #[fail(display = "We have some kind of error")]
    FailedToLoadFile(#[cause] io::Error),

	#[fail(display = "We have some kind of error: {}", _0)]
    SerdeError(#[cause] serde_json::Error),
}

header! { (XVaultToken, "X-Vault-Token") => [String] }

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let paths = get_file_paths(opt.directory.to_str().expect("Failed to convert path to string"))?;

    println!("{:?}", paths);

    for path in paths {
        let vault_path = path.strip_prefix(&opt.directory).map_err(Error::StripPrefix)?.parent().ok_or(Error::NoParent)?;
		let file_content = fs::read_to_string(&path).map_err(Error::FailedToLoadFile)?;

		// TODO: Should be a method probably
		let vault_value = match path.extension().and_then(OsStr::to_str) {
			Some("json") => {
				let _ : Value = serde_json::from_str(&file_content).unwrap();

				// If the parse succeeds it means the string is valid json, so we can just use it,
				// instead of serializing the struct
				file_content
			}
			Some("yaml") | Some("yml") => {
				let yaml_struct: Value = serde_yaml::from_str(&file_content).unwrap();

				serde_json::to_string(&yaml_struct).map_err(Error::SerdeError)?
			}
			_ => { "".to_string() }
		};



        update_vault_path(&opt.vault_uri, &opt.vault_token, vault_path.to_str().ok_or(Error::PathToStr)?, vault_value)?;
    }

    Ok(())
}

fn get_file_paths(path: &str) -> Result<Vec<PathBuf>, Error> {
    let direntries: Result<Vec<walkdir::DirEntry>, _> = WalkDir::new(path).into_iter().filter(|entry_result| {
        if let Ok(ref entry) = entry_result {
            entry.file_type().is_file()
        } else {
            false
        }
    }).collect();

    let direntries = direntries.map_err(Error::DirReadError)?;
    Ok(direntries.iter().map(|e| e.path().to_path_buf()).collect())
}


fn update_vault_path<S: ToString>(vault_uri: &Url, vault_token: &str, path: &str, value: S) -> Result<(), Error> {
   let url = vault_uri.join(&format!("{}{}", "/v1/", path)).map_err(Error::UriJoinError)?;
   println!("vault uri is {}", url);

	let client = reqwest::Client::new();
	let mut headers = Headers::new();
	headers.set(ContentType::json());
	headers.set(XVaultToken(vault_token.to_string()));

	let mut res = client.post(url)
					.headers(headers)
					.body(value.to_string())
					.send()
					.map_err(Error::VaultUpdate)?;

	println!("result was {}", res.text().expect("Error reading response"));
   	Ok(())
}

