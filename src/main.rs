use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{collections::HashMap, str::FromStr};

use tokio::net::{TcpListener, TcpStream};

use anyhow::Result;
use resp::Value;

mod resp;

struct Entry {
    item: String,
    time: Instant,
    ex: i32,
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db: Arc<Mutex<HashMap<String, Entry>>> = Arc::new(Mutex::new(HashMap::new())); // Wrap HashMap in Arc<Mutex<_>>

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

async fn handle_conn(stream: TcpStream, db: Arc<Mutex<HashMap<String, Entry>>>) {
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
                "dbsize" => dbsize_function(args, &db),
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

fn set_function(args: Vec<Value>, db: &Arc<Mutex<HashMap<String, Entry>>>) -> Value {
    match args.len() {
        2 => {
            let mut db = db.lock().unwrap(); // Lock the mutex to access the HashMap
            db.insert(
                args[0].value(),
                Entry {
                    item: args[1].value(),
                    time: Instant::now(),
                    ex: -1,
                },
            );

            Value::SimpleString(String::from_str("Ok").unwrap())
        }
        4 => {
            if args[2].value() == String::from("ex") {
                let ex = i32::from_str(args[3].value().as_str());
                match ex {
                    Ok(time) => {
                        let mut db = db.lock().unwrap(); // Lock the mutex to access the HashMap
                        db.insert(
                            args[0].value(),
                            Entry {
                                item: args[1].value(),
                                time: Instant::now(),
                                ex: time,
                            },
                        );

                        Value::SimpleString(String::from_str("Ok").unwrap())
                    }
                    Err(_) => Value::SimpleError(
                        String::from_str("Couldn't parse expiration time").unwrap(),
                    ),
                }
            } else {
                Value::SimpleError(String::from_str("Unexpected argument received").unwrap())
            }
        }
        _ => Value::SimpleError(String::from_str("Unexpected number of arguments").unwrap()),
    }
}

fn get_function(args: Vec<Value>, db: &Arc<Mutex<HashMap<String, Entry>>>) -> Value {
    if args.len() != 1 {
        return Value::SimpleError(String::from_str("Unexpected number of arguments").unwrap());
    }

    let mut db = db.lock().unwrap(); // Lock the mutex to access the HashMap
    let res = db.get(&args[0].value());
    match res {
        Some(entry) => {
            if entry.ex == -1 {
                Value::BulkString(entry.item.to_string())
            } else {
                let duration_passed = Instant::now().duration_since(entry.time);
                if duration_passed.as_secs() > entry.ex.try_into().unwrap() {
                    db.remove_entry(&args[0].value());
                    Value::NullBulk
                } else {
                    Value::BulkString(entry.item.to_string())
                }
            }
        }
        None => Value::NullBulk,
    }
}

fn dbsize_function(_args: Vec<Value>, db: &Arc<Mutex<HashMap<String, Entry>>>) -> Value {
    Value::SimpleString(db.lock().unwrap().len().to_string())
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
