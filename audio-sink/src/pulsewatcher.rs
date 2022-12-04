use anyhow::{anyhow, Result};
use log::{debug, error};
use pulse::{callbacks::ListResult, context::subscribe::InterestMaskSet};
use pulse::{context::Context, mainloop::api::Mainloop as MainloopTrait};
use pulse::{
    context::State,
    mainloop::standard::{IterateResult, Mainloop},
};
use std::{cell::RefCell, ops::Deref, rc::Rc, sync::Arc};
use tokio::sync::mpsc::UnboundedSender as Sender;

type PulseSender = Sender<PulseMessage>;

#[derive(Debug, PartialEq, Eq)]
pub enum PulseState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub enum PulseMessage {
    StateChanged(PulseState),
    SinksUpdated(Vec<String>),
}

pub struct PulseWatcher {
    mainloop: Arc<RefCell<Mainloop>>,
    context: Arc<RefCell<Context>>,
    sender: PulseSender,
}

impl PulseWatcher {
    pub fn new(sender: PulseSender) -> Result<PulseWatcher> {
        let mainloop =
            Arc::new(RefCell::new(Mainloop::new().ok_or_else(|| {
                anyhow!("Could not create a PulseAudio mainloop")
            })?));

        let context = Arc::new(RefCell::new(
            Context::new(mainloop.borrow().deref(), "ContextName")
                .ok_or_else(|| anyhow!("Could not create a PulseAudio context"))?,
        ));

        Ok(PulseWatcher {
            mainloop,
            context,
            sender,
        })
    }

    pub fn run(&self) -> Result<()> {
        debug!("Starting PulseWatcher");

        self.connect()?;
        self.subscribe_to_state_changes();
        self.subscribe_to_events();

        self.mainloop.borrow_mut().run().map_err(|(e, ret_val)| {
            anyhow!(
                "Cannot run PulseAudio loop, err: {}, ret_val: {:?}",
                e,
                ret_val
            )
        })?;

        Ok(())
    }

    fn connect(&self) -> Result<()> {
        debug!("Trying to connect to PulseAudio server");

        self.context
            .borrow_mut()
            .connect(None, pulse::context::FlagSet::NOAUTOSPAWN, None)?;

        loop {
            // debug!("Loop!");
            match self.mainloop.borrow_mut().iterate(false) {
                IterateResult::Quit(e) => {
                    return Err(anyhow!("Return value while connecting: {:?}", e));
                }
                IterateResult::Err(e) => {
                    return Err(anyhow!("Error while connecting: {:?}", e));
                }
                IterateResult::Success(_) => {}
            }

            match self.context.borrow().get_state() {
                pulse::context::State::Ready => {
                    debug!("Connected to PulseAudio");
                    if let Err(e) = self
                        .sender
                        .send(PulseMessage::StateChanged(PulseState::Connected))
                    {
                        error!("Error while sending state update: {:?}", e);
                    }
                    break;
                }
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    return Err(anyhow!(
                        "Context state failed or terminated after connecting, quitting."
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn subscribe_to_state_changes(&self) {
        let inner = Arc::new(self.mainloop.borrow().inner());
        let ctx = self.context.clone();
        let sender = self.sender.clone();

        self.context
            .borrow_mut()
            .set_state_callback(Some(Box::new(move || {
                let mut exit = false;
                let state = ctx.borrow().get_state();
                debug!("State changed to: {:?}", state);

                let message = if let State::Failed | State::Terminated = state {
                    exit = true;
                    Some(PulseMessage::StateChanged(PulseState::Disconnected))
                } else {
                    None
                };

                if let Some(msg) = message {
                    if let Err(e) = sender.send(msg) {
                        error!("Error while sending state update: {:?}", e);
                    }
                };

                if exit {
                    // Awful hack
                    let mut ml = Mainloop {
                        _inner: inner.as_ref().clone(),
                    };
                    ml.quit(pulse::def::Retval(0));
                }
            })));
    }

    fn subscribe_to_events(&self) {
        debug!(target: "PulseWatcher", "Subscribing to events");

        let sender = self.sender.clone();
        let ctx = self.context.clone();
        let callback = move |_, op, _| {
            if op == Some(pulse::context::subscribe::Operation::Changed) {
                return;
            }

            debug!("Subscribed event notification");

            let sinks = Rc::new(RefCell::new(vec![]));
            let sender = sender.clone();
            ctx.borrow_mut()
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
                        if let Err(e) = sender.send(PulseMessage::SinksUpdated(sinks)) {
                            error!("Error while sending sink update: {:?}", e);
                        }
                    }
                    ListResult::Error => {
                        error!("Error while retrieving sinks");
                    }
                });
        };

        let mut ctx = self.context.borrow_mut();
        ctx.subscribe(InterestMaskSet::SINK, |_| {});
        ctx.set_subscribe_callback(Some(Box::new(callback)));
    }
}
