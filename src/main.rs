use clap::Parser;
use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short)]
    time: u32,

    #[arg(long, short)]
    file: String,
}

/// Structure to hold the application state
struct State {
    /// Timeout duration for idle notification in milliseconds
    timeout: u32,

    /// Path to the file indicating idle state
    idle_mark_path: String,

    /// Wayland seat interface
    wl_seat: Option<wl_seat::WlSeat>,

    /// Idle notifier interface
    idle_notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,

    /// Queue handle for event queue
    qh: QueueHandle<Self>,
}

fn main() {
    let args = Args::parse();

    // Establish connection to the Wayland server
    let conn = Connection::connect_to_env().unwrap();
    // Create a new event queue
    let mut event_queue: EventQueue<State> = conn.new_event_queue();

    let qhandle = event_queue.handle();
    let display = conn.display();
    // Request Wayland registry to get global objects
    display.get_registry(&qhandle, ());

    // Initialize the application state
    let mut state = State {
        timeout: args.time * 60 * 1000,
        wl_seat: None,             // No Wayland seat interface initially
        idle_notifier: None,       // No idle notifier interface initially
        qh: qhandle,               // Queue handle for event queue
        idle_mark_path: args.file, // Path to the file indicating idled state
    };

    // Enter event dispatching loop
    loop {
        println!("Starting blocking dispatch");
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

impl State {
    /// Method to set up idle action based on seat and notifier availability
    fn set_idle_action(&self) {
        if let (Some(wl_seat), Some(idle_notifier)) =
            (self.wl_seat.as_ref(), self.idle_notifier.as_ref())
        {
            // Request idle notification from the idle notifier
            idle_notifier.get_idle_notification(
                self.timeout,
                &wl_seat,
                &self.qh,
                NotificationContext {}, // Empty context for the notification
            );
        }
    }
}

/// Structure representing notification context (currently empty)
struct NotificationContext {}

/// Event dispatching for idle notification events
impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, NotificationContext> for State {
    fn event(
        state: &mut Self,
        _idle_notification: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _ctx: &NotificationContext,
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Handle different idle notification events
        match event {
            ext_idle_notification_v1::Event::Idled => {
                // Create a file indicating idled state
                std::fs::File::create(&state.idle_mark_path).unwrap();
            }
            ext_idle_notification_v1::Event::Resumed => {
                // Remove the file indicating idled state
                std::fs::remove_file(&state.idle_mark_path).unwrap();
            }
            _ => println!("unknown"),
        };
    }
}

/// Event dispatching for idle notifier events
impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for State {
    fn event(
        _state: &mut Self,
        _idle_notifier: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _event: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No action required for idle notifier events
    }
}

/// Event dispatching for Wayland registry events
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // Handle different Wayland registry events
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match &interface[..] {
                "wl_seat" => {
                    println!("Wl seat set up");
                    // Bind the Wayland seat interface
                    let wl_seat = registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                    state.wl_seat = Some(wl_seat);

                    // Try to set idle action when notifier is available
                    state.set_idle_action();
                }
                "ext_idle_notifier_v1" => {
                    println!("Idle notifier set up");
                    // Bind the idle notifier interface
                    let idle_notifier = registry
                        .bind::<ext_idle_notifier_v1::ExtIdleNotifierV1, _, _>(name, 1, qh, ());
                    state.idle_notifier = Some(idle_notifier);

                    // Try to set idle action when notifier is available
                    state.set_idle_action();
                }
                _ => {} // No action required for other interfaces
            }
        }
    }
}

/// Event dispatching for Wayland seat events (currently no action defined)
impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No action required for Wayland seat events
    }
}
