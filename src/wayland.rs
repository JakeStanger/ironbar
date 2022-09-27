use std::cell::RefCell;
use std::rc::Rc;
use wayland_client::protocol::wl_output::{self, Event};
use wayland_client::{global_filter, Display as WlDisplay, GlobalManager, Main};

pub fn get_output_names() -> Vec<String> {
    // Connect to the server
    let display = WlDisplay::connect_to_env().unwrap();

    let mut event_queue = display.create_event_queue();

    let attached_display = (*display).clone().attach(event_queue.token());

    let outputs = Rc::new(RefCell::new(Vec::<String>::new()));

    let _globals = {
        let outputs = outputs.clone();
        GlobalManager::new_with_cb(&attached_display, {
            global_filter!([
                wl_output::WlOutput,
                4,
                move |output: Main<wl_output::WlOutput>, _: DispatchData| {
                    let outputs = outputs.clone();

                    output.quick_assign(move |_, event, _| match event {
                        Event::Name { name: title } => {
                            let outputs = outputs.clone();
                            outputs.as_ref().borrow_mut().push(title);
                        }
                        _ => {}
                    })
                }
            ])
        })
    };

    // A roundtrip synchronization to make sure the server received our registry
    // creation and sent us the global list
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    // for some reason we need to call this twice?
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    outputs.take()
}
