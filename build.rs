fn main(){
    println!("build.rs");
    for i in env!("Path").split(";") {
        if i.contains("ffmpeg") {
            println!("cargo:rustc-link-search={}", i);
        }
    }
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avutil");
}