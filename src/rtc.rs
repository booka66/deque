//! With --share through a tunnel, a watcher on the same network as the presenter
//! gets the talk straight from this machine, not round through Cloudflare:
//! the page, loaded through the tunnel, asks over it for a WebRTC data
//! channel, and connects to this machine's address on the network.
//!
//! The presenter's phone remote does the same, the other way: where it
//! points goes straight to this machine, as fast as the phone says it,
//! not a request at a time round through Cloudflare.
//!
//! It's as private as the page: what's sent is encrypted (DTLS), and the
//! page checks this end's key against the fingerprint in the answer it was
//! given over the tunnel, which it trusts. A watcher who can't reach this
//! machine (elsewhere, or on a network that keeps devices apart) never
//! connects, and keeps what the tunnel sends.

use crate::share::Hub;
use std::net::{IpAddr, UdpSocket};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use str0m::change::SdpOffer;
use str0m::channel::ChannelId;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};

/// Watchers connected straight at once, at most.
const MOST: usize = 100;
/// How long a watcher has to connect once it's asked.
const PATIENCE: Duration = Duration::from_secs(20);
/// What a watcher may have waiting unsent before it's let go: it falls
/// back to the tunnel.
const BEHIND: usize = 8 << 20;
/// How much goes in one message: what every browser takes.
const PIECE: usize = 16 << 10;

static OPEN: AtomicUsize = AtomicUsize::new(0);

/// Which end asked: a watcher, sent what's drawn, or the remote, whose
/// messages are what it asks for (as share::act takes them).
#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Watch,
    Remote,
}

/// The answer to a page's offer (its JSON, `{type, sdp}`), as JSON, and a
/// thread serving it from a UDP port of its own on `ip`.
pub fn answer(hub: &Hub, offer: &str, ip: IpAddr, side: Side) -> Result<String, String> {
    static CRYPTO: std::sync::Once = std::sync::Once::new();
    CRYPTO.call_once(|| str0m::crypto::from_feature_flags().install_process_default());
    let offer: SdpOffer = serde_json::from_str(offer).map_err(|e| e.to_string())?;
    if OPEN.fetch_add(1, Ordering::SeqCst) >= MOST {
        OPEN.fetch_sub(1, Ordering::SeqCst);
        return Err("too many".into());
    }
    let made = (|| {
        let mut rtc = RtcConfig::new().set_ice_lite(true).build(Instant::now());
        let socket = UdpSocket::bind((ip, 0)).map_err(|e| e.to_string())?;
        let addr = socket.local_addr().map_err(|e| e.to_string())?;
        rtc.add_local_candidate(Candidate::host(addr, "udp").map_err(|e| e.to_string())?);
        let answer = rtc.sdp_api().accept_offer(offer).map_err(|e| e.to_string())?;
        Ok::<_, String>((rtc, socket, serde_json::to_string(&answer).map_err(|e| e.to_string())?))
    })();
    match made {
        Ok((rtc, socket, answer)) => {
            let hub = hub.clone();
            std::thread::spawn(move || {
                serve(rtc, socket, &hub, side);
                OPEN.fetch_sub(1, Ordering::SeqCst);
            });
            Ok(answer)
        }
        Err(e) => {
            OPEN.fetch_sub(1, Ordering::SeqCst);
            Err(e)
        }
    }
}

/// One watcher: connected, then sent what's drawn as it's drawn, till it
/// goes, falls too far behind, or never connects. Or the remote: what it
/// asks for, done.
fn serve(mut rtc: Rtc, socket: UdpSocket, hub: &Hub, side: Side) {
    let born = Instant::now();
    let Ok(here) = socket.local_addr() else { return };
    let mut open: Option<(ChannelId, std::sync::mpsc::Receiver<std::sync::Arc<[u8]>>)> = None;
    let mut buf = vec![0u8; 2000];
    // The remote's channel's open: it's connected.
    let mut opened = false;
    loop {
        // What's been drawn, out to it.
        if let Some((id, rx)) = &open {
            let Some(mut ch) = rtc.channel(*id) else { return };
            loop {
                match rx.try_recv() {
                    Ok(b) => {
                        for piece in b.chunks(PIECE) {
                            if ch.write(true, piece).is_err() {
                                return;
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(_) => return,
                }
            }
            if ch.buffered_amount() > BEHIND {
                return;
            }
        }
        let until = match rtc.poll_output() {
            Ok(Output::Timeout(t)) => t,
            Ok(Output::Transmit(t)) => {
                let _ = socket.send_to(&t.contents, t.destination);
                continue;
            }
            Ok(Output::Event(e)) => {
                match e {
                    Event::ChannelOpen(id, _) if side == Side::Watch => open = Some((id, hub.viewer())),
                    Event::ChannelOpen(..) => opened = true,
                    Event::ChannelData(d) if side == Side::Remote && !d.binary => {
                        if let Ok(what) = std::str::from_utf8(&d.data) {
                            crate::share::act(hub, what);
                        }
                    }
                    Event::ChannelClose(_) | Event::IceConnectionStateChange(IceConnectionState::Disconnected) => return,
                    _ => {}
                }
                continue;
            }
            Err(_) => return,
        };
        if !rtc.is_alive() || (open.is_none() && !opened && born.elapsed() > PATIENCE) {
            return;
        }
        // A few milliseconds at most, so what's drawn goes out with little wait.
        let wait = until.saturating_duration_since(Instant::now()).min(Duration::from_millis(4));
        if wait.is_zero() {
            if rtc.handle_input(Input::Timeout(Instant::now())).is_err() {
                return;
            }
            continue;
        }
        let _ = socket.set_read_timeout(Some(wait));
        let input = match socket.recv_from(&mut buf) {
            Ok((n, source)) => match buf[..n].try_into() {
                Ok(contents) => Input::Receive(Instant::now(), Receive { proto: Protocol::Udp, source, destination: here, contents }),
                Err(_) => continue,
            },
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => Input::Timeout(Instant::now()),
            Err(_) => return,
        };
        if rtc.handle_input(input).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_offer_is_answered() {
        let share = crate::share::start(&crate::markup::Theme::default(), false, None, false).unwrap();
        let ip = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
        assert!(answer(&share.hub, "{}", ip, Side::Watch).is_err());
        assert!(answer(&share.hub, r#"{"type":"offer","sdp":"nonsense"}"#, ip, Side::Remote).is_err());
        assert_eq!(OPEN.load(Ordering::SeqCst), 0);
    }
}
