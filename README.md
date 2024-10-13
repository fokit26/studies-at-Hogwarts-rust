# Хогвадрз Лабораторис

**Выполнил проект:** Лойко Максим Андреевич, студент ФПМИ МФТИ

**Номер задачи:** 1

**Описание, как решение скомпилировать / запустить:**

*Клиент:*

`cargo run --bin client -- -h`

*Сервер:*

`cargo run --bin server -- -h`

*Админка:*

`cargo run --bin admin -- -h`

Это даст подсказку по аргументам командной строки.


**Описание принятых проектных решений:**

Решил писать на Rust. Я наткнулся на интересную библиотечку, которую использовал в решении: [message-io](https://crates.io/crates/message-io).

Решение разбито на три бинарника - сервер, клиент, админка. Сделано это с целью дать возможность отвечать на запросы сразу нескольким админам. Админка позволяет отвечать на запросы и смотреть статистику (как просилось в задании).

Для аутефикации админов используется токен, который служит как пароль (если его не указать, сгенерируется рандомный).

**Любая другая информация на ваше усмотрение:**