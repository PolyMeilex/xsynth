use std::sync::Arc;

use crate::core::{event::ChannelEvent, soundfont::SoundfontBase};

pub enum SynthEvent {
    Channel(u32, ChannelEvent),
    AllChannels(ChannelEvent)
}
