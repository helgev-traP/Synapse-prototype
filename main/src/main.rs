extern crate node;

#[tokio::main]
async fn main() {
    tokio::task::block_in_place(|| {
        println!("Hello, world!");
    });
}
