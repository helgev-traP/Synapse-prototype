mod input;
pub use input::*;
mod output;
pub use output::*;

/// --- Tests ---
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn input_minimum() {
        // todo
    }

    #[tokio::test]
    async fn input_memory() {
        // todo
    }

    #[tokio::test]
    async fn input_speed_benchmark() {
        // todo
    }

    #[tokio::test]
    async fn input_envelope() {
        // todo
    }
}
