use self::registers::dma_control_register::DmaControlRegister;

pub mod registers;

#[derive(Copy, Clone)]
pub struct DmaChannel {
  pub source_address: u32,
  pub destination_address: u32,
  pub dma_control: DmaControlRegister,
  pub word_count: u16,
  pub pending: bool
}

impl DmaChannel {
  pub fn new() -> Self {
    Self {
      source_address: 0,
      destination_address: 0,
      word_count: 0,
      dma_control: DmaControlRegister::from_bits_retain(0),
      pending: false
    }
  }
}