#[cfg(test)]
mod tests {
    #[test]
    fn protoc_exists() {
        let protoc = std::env::var_os("PROTOC").unwrap();
        std::fs::metadata(protoc);
    }
}
