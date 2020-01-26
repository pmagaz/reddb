use super::error;
use super::status::Status;
use super::store::{Document, ReadGuard, RedDbHashMap, WriteGuard};
use serde_json::{json, Value};
use std::result;
use uuid::Uuid;

pub type Result<T> = result::Result<T, error::RedDbError>;

#[derive(Debug)]
pub struct Query {}

impl Query {
  pub fn new() -> Result<Self> {
    println!("[RedDb] Setting up query");
    Ok(Self {})
  }

  pub fn get_id<'a>(&self, query: &'a Value) -> Result<&'a str> {
    //Fixme
    let _id = match query.get("_id").unwrap().as_str() {
      Some(_id) => _id,
      None => "",
    };
    Ok(_id)
  }

  pub fn get_uuid(&self, query: &Value) -> Result<Uuid> {
    let _id = self.get_id(query)?;
    let uuid = Uuid::parse_str(_id)?;
    Ok(uuid)
  }

  // pub fn insert(&mut self, query: Value) -> Result<Value> {
  //   let mut store = self.write_store()?;
  //   let doc = Document {
  //     data: query,
  //     status: Status::NotSaved,
  //   };
  //   let _id = Uuid::new_v4();
  //   let json_doc = json::to_jsondoc(&_id, &doc)?;
  //   store.insert(_id, doc);
  //   Ok(json_doc)
  // }

  // pub fn find_id(&self, query: &Value) -> Result<Value> {
  //   let store = self.read_store()?;
  //   let uuid = self.get_uuid(&query)?;
  //   let doc = store.get(&uuid).unwrap();
  //   let result = json::to_jsonresult(&uuid, &doc)?;
  //   Ok(result)
  // }

  //TODO unify find, update, delete
  // pub fn find(&self, query: &Value) -> Result<Value> {
  //   let store = self.read_store()?;
  //   let mut docs_founded = Vec::new();
  //   let query_map = query.as_object().unwrap();
  //   for (key, doc) in store.iter() {
  //     let mut properties_match: Vec<i32> = Vec::new();
  //     let num_properties = query_map.len();
  //     for (prop, value) in query_map.iter() {
  //       match &doc.data.get(prop) {
  //         Some(val) => {
  //           if val == &value {
  //             properties_match.push(1);
  //             if num_properties == properties_match.len() {
  //               docs_founded.push(json::to_jsonresult(&key, &doc)?)
  //             }
  //           }
  //         }
  //         None => (),
  //       };
  //     }
  //   }
  //   let result = Value::Array(docs_founded);
  //   Ok(result)
  // }

  pub fn update_status<'a>(&self, doc: &'a mut Document, status: Status) -> &'a mut Document {
    doc.status = status;
    doc
  }

  // pub fn find2<'a>(
  //     &self,
  //     store: &'a mut RwLockWriteGuard<RedDbHashMap>,
  //     query: Value,
  //     new_value: Value,
  // ) -> Result<Vec<(&'a Uuid, Value)>> {
  //     let query_map = query.as_object().unwrap();
  //     let num_properties = query_map.len();
  //     let result: Vec<(&Uuid, Value)> = store
  //         .iter_mut()
  //         .filter(|(_id, doc)| doc.status != Status::Deleted)
  //         .filter(|(_k, doc)| {
  //             let mut properties_match: Vec<i32> = Vec::new();
  //             for (prop, value) in query_map.iter() {
  //                 match doc.data.get(prop) {
  //                     Some(val) => {
  //                         if val == value {
  //                             properties_match.push(1);
  //                         }
  //                     }
  //                     None => (),
  //                 };
  //             }
  //             num_properties == properties_match.len()
  //         })
  //         .map(|(id, doc)| json::to_jsonresult(&id, &doc).unwrap())
  //         .collect();

  //     Ok(result)
  // }

  pub fn update<'a>(
    &self,
    store: &'a mut WriteGuard<RedDbHashMap>,
    query: Value,
    new_value: Value,
  ) -> Result<Vec<(&'a Uuid, &'a mut Document)>> {
    let query_map = query.as_object().unwrap();
    let num_properties = query_map.len();
    let docs: Vec<(&Uuid, &mut Document)> = store
      .iter_mut()
      .filter(|(_id, doc)| doc.status != Status::Deleted)
      .map(|(key, doc)| {
        let mut properties_match: Vec<i32> = Vec::new();
        for (prop, value) in query_map.iter() {
          match doc.data.get(prop) {
            Some(val) => {
              if val == value {
                properties_match.push(1);
                *doc.data.get_mut(prop).unwrap() = json!(new_value[prop]);
                if num_properties == properties_match.len() {
                  self.update_status(doc, Status::Updated);
                }
              }
            }
            None => (),
          };
        }
        (key, doc)
      })
      .collect();

    Ok(docs)
  }

  pub fn delete<'a>(
    &self,
    store: &'a mut WriteGuard<RedDbHashMap>,
    query: Value,
  ) -> Result<Vec<(&'a Uuid, &'a mut Document)>> {
    let query_map = query.as_object().unwrap();
    let num_properties = query_map.len();
    let docs: Vec<(&Uuid, &mut Document)> = store
      .iter_mut()
      .filter(|(_id, doc)| doc.status != Status::Deleted)
      .filter(|(_k, doc)| {
        let mut properties_match: Vec<i32> = Vec::new();
        for (prop, value) in query_map.iter() {
          match doc.data.get(prop) {
            Some(val) => {
              if val == value {
                properties_match.push(1);
              }
            }
            None => (),
          };
        }
        num_properties == properties_match.len()
      })
      .map(|(key, doc)| {
        self.update_status(doc, Status::Deleted);
        (key, doc)
      })
      .collect();

    Ok(docs)
  }
}

// pub fn get(&self) -> Result<()> {
//   let store = self.read_store().unwrap();
//   for (key, doc) in store.iter() {
//     println!("STORE DATA{:?}", doc);
//   }
//   Ok(())
// }
