pub enum Dtype {
    SimpleString,
    SimpleError,
    Integer,
    Array,
    BulkString,
}

pub fn parse(bytes: &[u8]) {
    let string = String::from_utf8_lossy(bytes);
    println!("{:?}", string);
}
