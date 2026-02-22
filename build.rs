fn main() {
    #[cfg(target_os = "windows")]
    {
        winres::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("Failed to embed Windows icon resource");
    }
}
