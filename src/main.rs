#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate formdata;

use formdata::FormData;
use rocket::data;
use rocket::data::FromDataSimple;
use rocket::http::hyper::header::Headers;
use rocket::http::HeaderMap;
use rocket::http::Status;
use rocket::Data;
use rocket::Request;
use std;
use std::fs;
use std::io;
use std::path::Path;

const UPLOADS_DIR: &'static str = "uploads/";
const SHA256_EXTENSION: &'static str = "sha256";

// struct Authenticated {}

struct RocketFormData {
    value: FormData,
}

fn from(header_map: &HeaderMap) -> Headers {
    let mut headers = Headers::new();
    for header in header_map.iter() {
        let header_value: Vec<u8> = header.value().as_bytes().to_owned();
        headers.append_raw(String::from(header.name()), header_value);
    }
    headers
}

impl FromDataSimple for RocketFormData {
    type Error = String;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let headers = from(request.headers());

        match formdata::read_formdata(&mut data.open(), &headers) {
            Ok(parsed_form) => {
                return data::Outcome::Success(RocketFormData { value: parsed_form })
            }
            _ => {
                return data::Outcome::Failure((
                    Status::BadRequest,
                    String::from("Failed to read fromdata"),
                ))
            }
        };
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/", format = "multipart/form-data", data = "<upload>")]
// fn upload_form(_auth: Authenticated, upload: RocketFormData) -> io::Result<String> {
fn upload_form(upload: RocketFormData) -> io::Result<String> {
    for (name, value) in upload.value.fields {
        println!("Posted field name={} value={}", name, value);
    }
    for (name, file) in upload.value.files {
        // file.do_not_delete_on_drop(); // don't delete temporary file
        let filename = match file.filename() {
            Ok(Some(original_filename)) => original_filename,
            _ => time::now().to_timespec().sec.to_string(),
        };
        println!(
            "Posted file fieldname={} name={} path={:?}",
            name, filename, file.path
        );
        let upload_location = Path::new(UPLOADS_DIR).join(&filename);
        match fs::copy(&file.path, &upload_location) {
            Ok(_) => return Ok(format!("Uploaded {}", filename)),
            Err(error) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Cannot write to {} directory due to {:?}",
                        UPLOADS_DIR, error
                    ),
                ))
            }
        };
    }
    return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "No files uploaded",
    ));
}

fn create_upload_directory() -> io::Result<bool> {
    fs::create_dir_all(UPLOADS_DIR)?;
    return Ok(true);
}

fn main() {
    let dir_path = std::env::current_dir();
    println!("current dir_path: {}", dir_path.unwrap().display());

    match create_upload_directory() {
        Err(error) => {
            eprintln!("Could not create ./{} directory: {}", UPLOADS_DIR, error);
            std::process::exit(1);
        }
        Ok(_) => {}
    }

    rocket::ignite()
        .mount("/", routes![index, upload_form])
        .launch();
}
