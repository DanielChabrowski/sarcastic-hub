use anyhow::anyhow;
use futures_util::SinkExt;
use futures_util::StreamExt;
use log::{debug, error, info};
use pulse::context::Context;
use pulse::mainloop::api::Mainloop as MainloopTrait;
use pulse::mainloop::standard::Mainloop;
use pulse::{callbacks::ListResult, mainloop::standard::IterateResult};
use std::{cell::RefCell, ops::Deref, rc::Rc, sync::Arc};
use tokio::sync::mpsc::unbounded_channel as channel;
use tokio::sync::mpsc::UnboundedSender as Sender;

#[derive(Debug, PartialEq)]
enum PAState {
    Disconnected,
}

fn connect(sender: Sender<HubMessage>) {
    debug!("Connecting to PulseAudio");

    let mainloop = Arc::new(RefCell::new(Mainloop::new().expect("Mainloop created")));
    let inner = Arc::new(mainloop.borrow().inner());

    let ctx = Arc::new(RefCell::new(
        Context::new(mainloop.borrow().deref(), "ContextName").expect("Context created"),
    ));

    let _ = ctx
        .borrow_mut()
        .connect(None, pulse::context::FlagSet::NOFLAGS, None)
        .map_err(|e| anyhow!("Err: {:?}", e));

    loop {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                error!("Iterate state was not success, quitting...");
                return;
            }
            IterateResult::Success(_) => {}
        }

        match ctx.borrow().get_state() {
            pulse::context::State::Ready => {
                debug!("Connected to PulseAudio");
                break;
            }
            pulse::context::State::Failed | pulse::context::State::Terminated => {
                error!("Context state failed/terminated, quitting...");
                return;
            }
            _ => {}
        }
    }

    use pulse::context::subscribe::InterestMaskSet;
    ctx.borrow_mut().subscribe(InterestMaskSet::SINK, |_| {});

    let ctx_c = ctx.clone();
    let sender_c = sender.clone();
    ctx.borrow_mut().set_state_callback(Some(Box::new(move || {
        let mut exit = false;
        let state = ctx_c.borrow().get_state();
        debug!("State changed to: {:?}", state);

        use pulse::context::State;
        let hub_message = if let State::Failed | State::Terminated = state {
            exit = true;
            Some(HubMessage::StateChanged(PAState::Disconnected))
        } else {
            None
        };

        let sender_c = sender_c.clone();
        if let Some(msg) = hub_message {
            sender_c.send(msg).expect("msg");
        };

        if exit {
            let mut ml = Mainloop {
                _inner: inner.as_ref().clone(),
            };
            ml.quit(pulse::def::Retval(0));
        }
    })));

    let ctx_c = ctx.clone();
    let sender_c = sender.clone();
    ctx.borrow_mut()
        .set_subscribe_callback(Some(Box::new(move |_, op, _| {
            if op == Some(pulse::context::subscribe::Operation::Changed) {
                return;
            }

            debug!("Subscription: {:?}", op);

            let sinks = Rc::new(RefCell::new(vec![]));
            let sender = sender_c.clone();
            ctx_c
                .borrow_mut()
                .introspect()
                .get_sink_info_list(move |result| match result {
                    ListResult::Item(item) => {
                        let sink_desc = item.description.as_ref().map(|cow| cow.to_string());
                        if let Some(desc) = sink_desc {
                            sinks.borrow_mut().push(desc);
                        }
                    }
                    ListResult::End => {
                        let sinks = sinks.borrow_mut().clone();
                        sender
                            .send(HubMessage::OutputDevices(sinks))
                            .expect("OutputDevices sent");
                    }
                    _ => {
                        // Error
                    }
                });
        })));

    let _ = mainloop.borrow_mut().run();
    info!("Receiver ended");
}

#[derive(Debug)]
enum HubMessage {
    StateChanged(PAState),
    OutputDevices(Vec<String>),
}

#[tokio::main]
async fn main() {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::init();
    }

    info!(
        "Starting {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let (sender, mut receiver) = channel::<HubMessage>();

    let server = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            debug!("Received {:?}", msg);
        }
    });

    let (ws_stream, _http_response) = tokio_tungstenite::connect_async(
        url::Url::parse("ws://127.0.0.1:9024").expect("Valid Hub url"),
    )
    .await
    .expect("WebSocket connection to Hub failed");

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let hub_connection = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            debug!("Message from Hub: {:?}", msg);
        }
    });

    ws_write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            "{ \"Register\": { \"name\": \"audio-sink\" } }".into(),
        ))
        .await
        .expect("Message send");

    let pulse_audio_connection = tokio::task::spawn_blocking(move || connect(sender));

    tokio::try_join!(server, hub_connection, pulse_audio_connection).expect("Joined");
}
