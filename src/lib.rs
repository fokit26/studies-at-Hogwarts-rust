use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ClientToServer {
  Register,
  Guess(i64),
  Ack(Uuid),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum GuessResult {
  Equal,
  Less,
  More,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ServerToClient {
  RegisterUUID(Uuid),
  ExperimentStart(Uuid),
  Answer(GuessResult, Uuid),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum AdminToServer {
  Auth(String),
  Start,
  Stats,
  WaitAnswers,
  SendAnswer { target: Uuid, answer: GuessResult },
}

/// Статистика по каждому участнику эксперимента
/// Отображает UUID -> количество попыток угадать число
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Stats(pub HashMap<Uuid, u64>);

/// Множество ожидающих ответа
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct WaitAnswers(pub HashMap<Uuid, i64>);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ServerToAdmin {
  Stats(Stats),
  WaitAnswers(WaitAnswers),
  ResultAuth(bool),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Message {
  Cts(ClientToServer),
  Stc(ServerToClient),
  Ats(AdminToServer),
  Sta(ServerToAdmin),
}
