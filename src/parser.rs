use std::{
    collections::HashMap,
    time::Instant
};

pub struct HttpRequest {
    pub method: String,
    pub host: String,
    pub version: String,
    pub path: String,
    pub connection: String,
    pub accept: Vec<String>,
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub bad_request: bool
}

impl HttpRequest {
    pub fn new(http_request: Vec<String>) -> (HttpRequest, Instant) {
        let mut method = String::from("");
        let mut path = String::from("");
        let mut host: String = String::from("");
        let mut version: String = String::from("");
        let mut connection: String = String::from("");
        let mut accept: Vec<String> = vec![];
        let mut params: HashMap<String, String> = HashMap::new();
        let mut headers: HashMap<String, String> = HashMap::new();
        let mut bad_request: bool = false;
        
        let mut count = 0;
        let request_start = Instant::now();
        
        for line in http_request {
            if line == "\r\n" { break }

            if count == 0 {
                for (i, val) in line.split_whitespace().enumerate() {
                    match i {
                        0 => method = val.to_string(),
                        1 => {
                            let path_option = val.split_once("?");
                            let (_, params_string) = match path_option {
                                Some((path_string, params_string)) => {
                                    path = path_string.to_string();
                                    (path_string, params_string)
                                },
                                _ => {
                                    path = val.to_string();
                                    ("", "")
                                }
                            };
                            if !params_string.is_empty() {
                                params = parse_params(&params_string);
                            }
                        },
                        2 => version = val.to_string(),
                        _ => ()
                    }
                };
            } else {
                let key_pair = line.split_once(":");
            
                let (key, pair) = match key_pair {
                    Some((key,pair)) => (key, pair),
                    _ => {
                        bad_request = true;
                        ("", "")
                    }
                };

                let key_clean = key.trim().to_string();
                let pair_clean = pair.trim().to_string();

                headers.insert(key_clean.clone(), pair_clean.clone());

                match key_clean.as_str() {
                    "Host" => host = pair_clean,
                    "Connection" => connection = pair_clean,
                    "Accept" => accept = parse_accept(&pair_clean),
                    _ => ()
                }
            }
            count += 1;
        }

        (HttpRequest {
            method,
            path,
            host,
            version,
            connection,
            accept,
            params,
            headers,
            bad_request,
        }, request_start)
    }
}


fn parse_params(params_string: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();

    let params_iter = params_string.split("&");

    for param in params_iter {
        let param_option = param.split_once("=");
        let (key, pair) = match param_option {
            Some((key, pair)) => (key, pair),
            _ => ("", "")
        };

        let key_clean = key.trim().to_string();
        let pair_clean = pair.trim().to_string();

        if !key_clean.is_empty() && !pair_clean.is_empty() {
            params.insert(key_clean.clone(), pair_clean.clone());
        }
    }

    params
}

fn parse_accept(accept_string: &str) -> Vec<String> {
    let mut accept = vec![];
    let accept_iter = accept_string.split(",");

    for accept_type in accept_iter {
        accept.push(accept_type.to_string());
    }

    accept
} 