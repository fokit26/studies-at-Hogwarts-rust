use std::{
  collections::HashMap,
  net::{IpAddr, Ipv4Addr},
};

use hogwarts_guess::{
  AdminToServer, ClientToServer, Message, ServerToAdmin, ServerToClient, Stats, WaitAnswers,
};

use clap::Parser;
use message_io::{
  network::{Endpoint, Transport},
  node::{self, NodeHandler},
};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "'Хогвартс Лабораторис' сервер")]
#[command(version = "0.1")]
#[command(about = "Сервер эксперимента о угадывании чисел", long_about = None)]
struct Cli {
  #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))]
  address: IpAddr,
  #[arg(short, long, default_value_t = 6969)]
  port: u16,
  #[arg(short = 't', long)]
  auth_token: Option<String>,
}

#[derive(PartialEq, Eq)]
enum EndpointStatus {
  JustConnected,
  AuthedAsUser(Uuid),
  AuthedAsAdmin,
}

struct ServerState {
  auth_token: String,
  clients: HashMap<Endpoint, EndpointStatus>,
  handler: NodeHandler<()>,
  uuids_users: HashMap<Uuid, Endpoint>,
  stat_users: Stats,
  waiting_users: WaitAnswers,
}

impl ServerState {
  fn new(auth_token: String, handler: NodeHandler<()>) -> Self {
    Self {
      auth_token,
      clients: HashMap::new(),
      handler,
      stat_users: Stats(HashMap::new()),
      waiting_users: WaitAnswers(HashMap::new()),
      uuids_users: HashMap::new(),
    }
  }

  fn register(&mut self, endpoint: Endpoint) {
    self.clients.insert(endpoint, EndpointStatus::JustConnected);
  }

  fn unregister(&mut self, endpoint: Endpoint) {
    self.clients.remove(&endpoint);
  }

  fn exec_message(&mut self, endpoint: Endpoint, message: Message) {
    match message {
      Message::Stc(_) | Message::Sta(_) => {
        return println!("Невалидная категория сообщения: эндпоинт({})", endpoint)
      }
      Message::Cts(cts_msg) => {
        self.exec_client_message(endpoint, cts_msg);
      }
      Message::Ats(ats_msg) => {
        self.exec_admin_message(endpoint, ats_msg);
      }
    }
  }

  fn exec_admin_message(&mut self, endpoint: Endpoint, message: AdminToServer) {
    if let hogwarts_guess::AdminToServer::Auth(auth_token) = message {
      if self.auth_token == auth_token {
        println!("Аутефицирован: эндпоинт({})", endpoint);
        self.clients.insert(endpoint, EndpointStatus::AuthedAsAdmin);
        self.handler.network().send(
          endpoint,
          &bincode::serialize(&Message::Sta(ServerToAdmin::ResultAuth(true))).unwrap(),
        );
      } else {
        println!("Неудачно аутефицирован: эндпоинт({})", endpoint);
        self.handler.network().send(
          endpoint,
          &bincode::serialize(&Message::Sta(ServerToAdmin::ResultAuth(false))).unwrap(),
        );
      }
      return;
    };
    if !self
      .clients
      .get(&endpoint)
      .is_some_and(|v| *v == EndpointStatus::AuthedAsAdmin)
    {
      return println!("Доступ к админке без аутефикации: эндпоинт({})", endpoint);
    }
    match message {
      hogwarts_guess::AdminToServer::Start => {
        println!("Рассылка начала игры начата");
        for (endpoint, client) in self.clients.iter() {
          match client {
            EndpointStatus::AuthedAsUser { .. } => {
              let msg_uuid = Uuid::new_v4();
              println!(
                "  Отправка: эндпоинт({}) & сообщение({})",
                endpoint, msg_uuid
              );
              self.handler.network().send(
                endpoint.clone(),
                &bincode::serialize(&Message::Stc(ServerToClient::ExperimentStart(msg_uuid)))
                  .unwrap(),
              );
            }
            _ => {}
          }
        }
        println!("Рассылка начала игры закончена")
      }
      hogwarts_guess::AdminToServer::Stats => {
        println!("Отправка статистики: эндпоинт({})", endpoint);
        self.handler.network().send(
          endpoint,
          &bincode::serialize(&Message::Sta(ServerToAdmin::Stats(self.stat_users.clone())))
            .unwrap(),
        );
      }
      hogwarts_guess::AdminToServer::WaitAnswers => {
        println!("Отправка списка ожидания: эндпоинт({})", endpoint);
        self.handler.network().send(
          endpoint,
          &bincode::serialize(&Message::Sta(ServerToAdmin::WaitAnswers(
            self.waiting_users.clone(),
          )))
          .unwrap(),
        );
      }
      hogwarts_guess::AdminToServer::SendAnswer { target, answer } => {
        println!(
          "Принят ответ на попытку: эндпоинт({}) & answer({:?}) @ таргет({})",
          endpoint, answer, target
        );
        if let Some(trg_endpoint) = self.uuids_users.get(&target) {
          let msg_uuid = Uuid::new_v4();
          println!(
            "Отправка: эндпоинт({}) & сообщение({})",
            trg_endpoint, msg_uuid
          );
          self.handler.network().send(
            trg_endpoint.clone(),
            &bincode::serialize(&Message::Stc(ServerToClient::Answer(answer, msg_uuid))).unwrap(),
          );
        } else {
          println!("Клиент не найден!");
        }
      }
      hogwarts_guess::AdminToServer::Auth(_) => unreachable!(), // Было обработано раннее
    }
  }

  fn exec_client_message(&mut self, endpoint: Endpoint, message: ClientToServer) {
    match message {
      ClientToServer::Register => {
        let new_uuid = Uuid::new_v4();
        println!(
          "Зарегистрирован юзер: эндпоинт({}) & уид({})",
          endpoint, new_uuid
        );
        self
          .clients
          .insert(endpoint.clone(), EndpointStatus::AuthedAsUser(new_uuid));
        self.handler.network().send(
          endpoint,
          &bincode::serialize(&Message::Stc(ServerToClient::RegisterUUID(new_uuid))).unwrap(),
        );
      }
      ClientToServer::Guess(guess) => {
        println!("Попытка: эндпоинт({}) & попытка({})", endpoint, guess);
        if let Some(EndpointStatus::AuthedAsUser(uuid)) = self.clients.get(&endpoint) {
          self.waiting_users.0.insert(uuid.clone(), guess);
        } else {
          println!("Не удалось найти юзера");
        }
      }
      ClientToServer::Ack(uuid) => {
        println!("Получено подтверждение: уид({})", uuid);
      }
    }
  }
}

fn main() {
  let cli = Cli::parse();

  let auth_token = cli.auth_token.unwrap_or_else(|| Uuid::new_v4().to_string());
  println!("Токен аутефикации: {}", auth_token);

  let (handler, listener) = node::split::<()>();
  match handler
    .network()
    .listen(Transport::Tcp, (cli.address, cli.port))
  {
    Ok((id, real_addr)) => println!("Слушаем на {} & id({})", real_addr, id),
    Err(err) => return println!("Не удалось открыть эндпоинт: {:?}", err),
  }

  let mut state = ServerState::new(auth_token, handler);

  listener.for_each(|event| match event.network() {
    message_io::network::NetEvent::Connected(_, _) => unreachable!(), // Вызывается только с клиентской стороны
    message_io::network::NetEvent::Accepted(endpoint, _) => {
      println!("Клиент подключился: эндпоинт({})", endpoint);
      state.register(endpoint);
    }
    message_io::network::NetEvent::Message(endpoint, data) => {
      let msg: Message = match bincode::deserialize(data) {
        Err(err) => return println!("Не удалось распарсить сообщение: {:?}", err),
        Ok(msg) => msg,
      };
      state.exec_message(endpoint, msg);
    }
    message_io::network::NetEvent::Disconnected(endpoint) => {
      println!("Клиент отключился: эндпоинт({})", endpoint);
      state.unregister(endpoint);
    }
  });
}
