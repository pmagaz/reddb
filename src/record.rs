use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Record<T> {
  pub _id: Uuid,
  pub data: T,
}

pub trait Document<T>: Clone {
  fn get_data(&self) -> &T;
  // fn get_vec(&self) -> &T;
}

impl<T> Document<T> for Record<T>
where
  T: Clone,
{
  fn get_data(&self) -> &T {
    &self.data
  }
}
