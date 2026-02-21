use crate::gui::state::PtyEvent;

/// Spawns a dedicated PTY reader thread that reads from the given reader and sends
/// `PtyEvent::Data` / `PtyEvent::Exited` events through the channel, waking the
/// event loop proxy after each send.
pub(in crate::gui) fn spawn_pty_reader(
    mut reader: Box<dyn std::io::Read + Send>,
    tx: std::sync::mpsc::Sender<PtyEvent>,
    proxy: winit::event_loop::EventLoopProxy<()>,
    tab_id: u64,
    pane_id: u64,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(format!("pty-reader-{}-{}", tab_id, pane_id))
        .spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send(PtyEvent::Exited { tab_id, pane_id });
                        let _ = proxy.send_event(());
                        break;
                    }
                    Err(err) => {
                        eprintln!("PTY read error for tab {tab_id} pane {pane_id}: {err}");
                        let _ = tx.send(PtyEvent::Exited { tab_id, pane_id });
                        let _ = proxy.send_event(());
                        break;
                    }
                    Ok(n) => {
                        if tx
                            .send(PtyEvent::Data {
                                tab_id,
                                pane_id,
                                bytes: buf[..n].to_vec(),
                            })
                            .is_err()
                        {
                            eprintln!("PTY reader {}-{}: channel disconnected", tab_id, pane_id);
                            break;
                        }
                        let _ = proxy.send_event(());
                    }
                }
            }
        })?;
    Ok(())
}
