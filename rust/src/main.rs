use std::sync::{Arc, Mutex};
use std::{collections::HashMap, str::FromStr};

use tokio::net::{TcpListener, TcpStream};

use anyhow::Result;
use resp::Value;

mod resp;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new())); // Wrap HashMap in Arc<Mutex<_>>

    loop {
        let stream = listener.accept().await;
        match stream {
            Ok((stream, _)) => {
                println!("Accepted new connection!");
                let db_clone = Arc::clone(&db); // Clone Arc
                tokio::spawn(async move { handle_conn(stream, db_clone).await });
                // Pass cloned Arc
            }

            Err(e) => {
                println!("Error connecting {}", e);
            }
        }
    }
}

async fn handle_conn(stream: TcpStream, db: Arc<Mutex<HashMap<String, String>>>) {
    let mut handler = resp::RespHandler::new(stream);

    println!("Starting read loop");

    loop {
        let value = handler.read_value().await.unwrap();

        println!("Got value {:?}", value);

        let response = if let Some(v) = value {
            let (command, args) = extract_command(v).unwrap();
            match command.as_str() {
                "ping" => Value::SimpleString("PONG".to_string()),
                "echo" => args.first().unwrap().clone(),
                "set" => set_function(args, &db),
                "get" => get_function(args, &db),
                "command" => Value::SimpleString("Ok".to_string()),
                c => panic!("Cannot handle command {}", c),
            }
        } else {
            break;
        };

        println!("Sending value {:?}", response);

        handler.write_value(response).await.unwrap();
    }
}

fn set_function(args: Vec<Value>, db: &Arc<Mutex<HashMap<String, String>>>) -> Value {
    if args.len() != 2 {
        return Value::SimpleError(String::from_str("Unexpected number of arguments").unwrap());
    }

    let mut db = db.lock().unwrap(); // Lock the mutex to access the HashMap
    db.insert(args[0].value(), args[1].value());

    Value::SimpleString(String::from_str("Ok").unwrap())
}

fn get_function(args: Vec<Value>, db: &Arc<Mutex<HashMap<String, String>>>) -> Value {
    if args.len() != 1 {
        return Value::SimpleError(String::from_str("Unexpected number of arguments").unwrap());
    }

    let db = db.lock().unwrap(); // Lock the mutex to access the HashMap
    let res = db.get(&args[0].value());
    match res {
        Some(item) => Value::BulkString(item.to_string()),
        None => Value::NullBulk,
    }
}

fn extract_command(value: Value) -> Result<(String, Vec<Value>)> {
    match value {
        Value::Array(a) => Ok((
            unpack_bulk_str(a.first().unwrap().clone())?,
            a.into_iter().skip(1).collect(),
        )),
        _ => Err(anyhow::anyhow!("Unexpected command format")),
    }
}

fn unpack_bulk_str(value: Value) -> Result<String> {
    match value {
        Value::BulkString(s) => Ok(s.to_lowercase()),
        _ => Err(anyhow::anyhow!("Expected command to be a bulk string")),
    }
}
