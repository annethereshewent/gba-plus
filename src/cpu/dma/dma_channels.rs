use super::dma_channel::DmaChannel;

#[derive(Copy, Clone)]
pub struct DmaChannels {
  pub channels: [DmaChannel; 4]
}

pub enum AddressType {
  Low,
  High
}

impl DmaChannels {
  pub fn new() -> Self {
    Self {
      channels: [DmaChannel::new(); 4]
    }
  }

  pub fn notify_gpu_event(&mut self, timing: u16) {

  }

  pub fn set_source_address(&mut self, channel_id: usize, value: u16, address_type: AddressType) {
    match address_type {
      AddressType::Low => {
        self.channels[channel_id].source_address = (self.channels[channel_id].source_address & 0xffff0000) | (value as u32);
      }
      AddressType::High => {
        self.channels[channel_id].source_address = (self.channels[channel_id].source_address & 0xffff) | (value as u32) << 16
      }
    }
  }

  pub fn set_destination_address(&mut self, channel_id: usize, value: u16, address_type: AddressType) {
    match address_type {
      AddressType::Low => {
        self.channels[channel_id].destination_address = (self.channels[channel_id].destination_address & 0xffff0000) | (value as u32);
      }
      AddressType::High => {
        self.channels[channel_id].destination_address = (self.channels[channel_id].destination_address & 0xffff) | (value as u32) << 16
      }
    }
  }
}