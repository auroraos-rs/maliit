# Maliit lib-rs

## Описание
Эта библиотека это что-то типа maliit-glib но написана на Rust и имеет меньше возможностей.

> [!WARNING]
> Возможны баги и краши, т.к. интерфейс фреймворка maliit описан очень плохо, из-за чего разработка велась наощупь.

> [!NOTE]
> Разработка велась под ОС Аврора, работоспособность в десктопных окружениях linux не гарантируется.

## Возможности
- [x] Вызов экранной клавиатуры
- [x] Сокрытие экранной клавиатуры
- [x] Получение событий нажатия на клавиши экранной клавиатуры
- [x] Сброс состояния ввода
- [x] Установка языка клавиатуры
- [ ] Отправка текста для подсказок с клавиатуры

## Использование

```rust
use maliit::{InputMethod, MaliitError};
use std::time::Duration;

fn main() -> Result<(), MaliitError> {
    let mut im = InputMethod::new()?;

    // Установка языка перед показом клавиатуры
    im.set_language("ru")?;

    // Показать клавиатуру
    im.show()?;

    // Обработка событий через callback
    im.process_events_with(Duration::from_millis(100), |event| {
        println!("Событие: {:?}", event);
    })?;

    // Скрыть клавиатуру
    im.hide()?;

    Ok(())
}
```

## API

Все публичные методы `InputMethod` возвращают `Result<..., MaliitError>` вместо паники при ошибках D-Bus.

### Основные методы

| Метод | Описание |
|---|---|
| `InputMethod::new()` | Подключение к Maliit серверу по D-Bus |
| `show()` | Показать экранную клавиатуру и начать обработку событий |
| `hide()` | Скрыть экранную клавиатуру и остановить обработку событий |
| `reset()` | Сбросить состояние ввода |
| `set_language(lang)` | Установить язык клавиатуры |

### Обработка событий

```rust
// Через callback (рекомендуется)
im.process_events_with(timeout, |event| {
    match event {
        InputMethodEvent::Text(text) => { /* текст введён */ }
        InputMethodEvent::Key { key, pressed } => { /* нажата клавиша */ }
        InputMethodEvent::AreaChanged(x, y, w, h) => { /* изменилась область клавиатуры */ }
    }
})?;

// Или пакетная обработка
let events = im.poll_events(timeout)?;
for event in events { ... }
```

## Ошибки

Все ошибки представлены типом `MaliitError`:

```rust
pub enum MaliitError {
    Dbus(dbus::Error),
    NotAvailable,
}
```
