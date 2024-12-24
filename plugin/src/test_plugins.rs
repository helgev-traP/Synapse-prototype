use node::framework::NodeFramework;

pub mod i64_to_string;
pub mod output_frame_count;

pub fn give_all_plugins() -> Vec<Box<dyn NodeFramework>> {
    vec![
        Box::new(output_frame_count::CurrentFrameCount),
        Box::new(i64_to_string::I64ToString),
    ]
}
