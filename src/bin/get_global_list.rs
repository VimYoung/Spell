use smithay_client_toolkit::reexports::client::{
    globals::{registry_queue_init, Global, GlobalListContents},
    protocol::wl_registry,
    Connection, Dispatch, QueueHandle,
};

// We need a State struct even if we don't use it
struct State;

// You need to provide a Dispatch<WlRegistry, GlobalListContents> impl for your app
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        state: &mut State,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        // The `GlobalListContents` is a container with an up-to-date list of
        // the currently existing globals
        data: &GlobalListContents,
        conn: &Connection,
        qhandle: &QueueHandle<State>,
    ) {
        /*
         * This simple program does not handle dynamic global events,
         * so we don't do anything here.
         */
    }
}

fn main() {
    let connection = Connection::connect_to_env().unwrap();
    let (globals, queue) = registry_queue_init::<State>(&connection).unwrap();

    // Print the contents of the list
    // We cannot iterate the list directly because of thread-safety constraints,
    // so we clone it and iterate on the returned Vec
    for global in globals.contents().clone_list() {
        println!(
            "Global #{} with interface \"{}\" and version {}",
            global.name, global.interface, global.version
        );
    }
}
