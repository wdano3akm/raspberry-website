use std::{
    convert::Infallible,
    env::args,
    collections::HashMap,
    io,
    path::PathBuf,
    process::exit,
    sync::Arc,
    process::Command, println,
    };
use http::{
    Method, StatusCode
    };
use hyper::{
    {Body, Request, Response, Server},
    service::{make_service_fn, service_fn},
    };
use tokio::{
    sync::oneshot::Sender,
    {fs, fs::read_dir},
    };
use chrono::prelude::*;


#[tokio::main]
async fn main() {
    NonBlockingStd::reload_css().expect("shit");
    match server().await {
        Ok(_) => {},
        Err(e) => println!("{e}"),
    };
}

pub async fn server() -> Result<(), Infallible> {

    
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let mut folder = NonBlockingStd::new().await;
    let path = Arc::new(folder.folder());
    let make_svc = make_service_fn(move |_|{
        let  path = path.clone();
        let service = service_fn(move |req| { 
            println!("[{}]: {}{:?}\n",Local::now(), req.uri(), req.headers());
            serve_request(req, path.clone())
        });
        async move {Ok::<_, Infallible>(service) }
    });

    let addr = ([127, 0, 0, 1], 7878).into();
    let server = Server::bind(&addr).serve(make_svc);

    // create oneshot mpsc to sent signal to gracefully shutdown server from CLI
    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();}
    );
    tokio::spawn(async move{
        // share transmitter with function
        folder.stdin_loop(tx).await;
    });

    if let Err(e) = graceful.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

async fn serve_request(req: Request<Body>, folder: Arc<PathBuf>) -> Result<Response<Body>, Infallible> {
    let (list_files, urls) = NonBlockingStd::search_dir(&folder).await;
    if req.method() == &Method::GET && req.uri().path() == "/" {
             let body = fs::read_to_string("templates/templates/main.html").await.unwrap();
            return Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(body))
                .unwrap())         
    }

    if req.method() == &Method::GET && req.uri().path() == "/main.css" {
        println!("here");
        let body = fs::read_to_string("templates/templates/main.css").await.unwrap();
        return Ok(Response::builder()
            .header("Content-Type", "text/css")
            .body(Body::from(body))
            .unwrap());
    }
    let request = &req.uri().path().to_string();
    if req.method() == &Method::GET && urls.contains(&request) {
        let req = request;

        

        let body = fs::read_to_string(list_files.get(req).unwrap()).await.unwrap();
        Ok(Response::builder()
            .header("Content-Type", "text/html") 
            .body(Body::from(body))
            .unwrap())
    } else { error(404).await }
    

}

pub async fn error(code: u16) -> Result<Response<Body>, Infallible> {
    let (response, status): (String, StatusCode) =  match code {
        500 => ("server issues".to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        404 => {
            let response = match fs::read_to_string("templates/templates/404.html").await {
                Ok(file) => file,
                Err(_) => {
                    return Ok(Response::new(Body::from("Well, that wasn't supposed to happen")))
                },
            };
            let response = response;
            (response, StatusCode::NOT_FOUND)}
        _ => panic!("status code not accounted for"),
    }; 
    let mut response :Response<Body> = Response::new(Body::from(response));
    *response.status_mut() = status;
    Ok(response)
} 

struct NonBlockingStd {
    // the keys of "list files" and the vector "possible urls" contain the same data,
    // it avoids having to ".into_keys()" the HashMap constantly
    folder: PathBuf,
}

unsafe impl Send for NonBlockingStd {}

impl NonBlockingStd {
    pub async fn new() -> Self {
        
        let args :Vec<String> = args().collect();
        let path = PathBuf::from(&args[1]);
        if args.len() != 2 || !path.is_dir() {
            eprintln!("ERROR: incorrect arguments");
            eprintln!("Please specify the relative path of a valid folder");
            eprintln!("Usage: ./website <folder>");
            exit(1)
        } 
        
        NonBlockingStd{  folder: path  }

    }


    pub async fn stdin_loop( &mut self, tx: Sender<()> ) {
        // loop indefinetely and read from stdin in separate thread to avoid blocking server
        loop { 
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();

            match buffer.trim() {
               "exit"  => {

                    // if we need to terminate the program we send the signal
                    // and then break the loop since we don't need to iterate anymore
                    println!("Gracefully shutting down the server now");
                    tx.send(()).unwrap();
                    break;
                },
                "reload" => {
                    Self::reload_css().expect("shit")
                },
                _ => continue
            }
        }
    }

    fn folder(&self) -> PathBuf {
        self.folder.clone()
    }

    async fn search_dir(path: &PathBuf) -> (HashMap<String, PathBuf>, Vec<String>) {
         let mut dir = read_dir(&path).await.unwrap();
            let mut hashm:HashMap<String, PathBuf> = HashMap::new();
            let mut list: Vec<String> = Vec::new();
           
            while let Some(entry) = dir.next_entry().await.unwrap() {
                let path = entry.path();
                let string = &path.into_os_string().into_string().unwrap()
                    //reverse so that first "." must be the type of file
                    .trim().chars().rev().collect::<String>();
                let end = &string
                    .trim()
                    [..=string.find(".").unwrap()];
                // check for html or css
                if end == "lmth." || end == "ssc." {
                //back to normal only if necessary
                //hashmap has extention to use in url + "/" (String) and path to retrieve file (PathBuf)
                    let index = string.find("/").unwrap();
                    let name_file = string[..=index].chars().rev().collect::<String>();
                    let path = PathBuf::from(string.chars().rev().collect::<String>());
                    
                    list.push(name_file.to_owned());
                    hashm.insert(name_file, path);
                }           
        }
    (hashm, list)
}
    fn reload_css() -> Result<(), String>{
        Command::new("ls").spawn().unwrap();
       
        match Command::new("python")
            .current_dir("python_scripts")
            .arg("force_reload_css.py") 
            .output() {
                Ok(_) => return Ok(()),
                Err(e) => return Err(e.to_string()),
            }
    }
}
