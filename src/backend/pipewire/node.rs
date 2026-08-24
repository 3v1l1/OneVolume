use pipewire::node::NodeInfoRef;

/// Print the handful of properties that actually matter for OneVolume,
/// instead of dumping every property PipeWire attaches to a node.
pub fn print_info(info: &NodeInfoRef) {
    let props = info.props();

    let get = |key: &str| -> &str { props.and_then(|p| p.get(key)).unwrap_or("(unknown)") };

    println!();
    println!("Application:  {}", get("application.name"));
    println!("Media:        {}", get("media.class"));
    println!("Node:         {}", get("node.name"));
    println!("Description:  {}", get("node.description"));
    println!("State:        {:?}", info.state());
    println!();
}
