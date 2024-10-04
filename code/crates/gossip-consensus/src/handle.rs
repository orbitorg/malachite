use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::task;

use crate::{Channel, CtrlMsg, Event};

pub struct RecvHandle {
    rx_event: mpsc::Receiver<Event>,
}

impl RecvHandle {
    pub async fn recv(&mut self) -> Option<Event> {
        self.rx_event.recv().await
    }
}

pub struct CtrlHandle {
    tx_ctrl: mpsc::Sender<CtrlMsg>,
    task_handle: task::JoinHandle<()>,
}

impl CtrlHandle {
    pub async fn publish(&self, channel: Channel, data: Bytes) -> Result<(), eyre::Report> {
        self.tx_ctrl.send(CtrlMsg::Publish(channel, data)).await?;
        Ok(())
    }

    pub async fn wait_shutdown(self) -> Result<(), eyre::Report> {
        self.shutdown().await?;
        self.join().await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), eyre::Report> {
        self.tx_ctrl.send(CtrlMsg::Shutdown).await?;
        Ok(())
    }

    pub async fn join(self) -> Result<(), eyre::Report> {
        self.task_handle.await?;
        Ok(())
    }
}

pub struct Handle {
    recv: RecvHandle,
    ctrl: CtrlHandle,
}

impl Handle {
    pub fn new(
        tx_ctrl: mpsc::Sender<CtrlMsg>,
        rx_event: mpsc::Receiver<Event>,
        task_handle: task::JoinHandle<()>,
    ) -> Self {
        Self {
            recv: RecvHandle { rx_event },
            ctrl: CtrlHandle {
                tx_ctrl,
                task_handle,
            },
        }
    }

    pub fn split(self) -> (RecvHandle, CtrlHandle) {
        (self.recv, self.ctrl)
    }

    pub async fn recv(&mut self) -> Option<Event> {
        self.recv.recv().await
    }

    pub async fn broadcast(&self, channel: Channel, data: Bytes) -> Result<(), eyre::Report> {
        self.ctrl.publish(channel, data).await
    }

    pub async fn wait_shutdown(self) -> Result<(), eyre::Report> {
        self.ctrl.wait_shutdown().await
    }

    pub async fn shutdown(&self) -> Result<(), eyre::Report> {
        self.ctrl.shutdown().await
    }

    pub async fn join(self) -> Result<(), eyre::Report> {
        self.ctrl.join().await
    }
}
