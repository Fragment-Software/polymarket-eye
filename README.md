# Polymarket Eye

## English

### Installation

#### Prerequisites

- **Rust** : Ensure you have Rust installed. You can download and install Rust from [https://www.rust-lang.org/tools/install]().

### Build

Clone the repository and build the project:

```
git clone https://github.com/Fragment-Software/polymarket-eye.git
cd polymarket-eye
cargo build --release
```

### Configuration

Before running the software, configure the necessary files:

1. **private_keys.txt** : Add your private keys to `data/private_keys.txt`.
2. **proxies.txt** : Add your proxies to `data/proxies.txt`.

### Running

Execute the built binary:

`cargo run --release`

### Output

After running, the output will be saved to `data/out.txt` in the following format:

`wallet_address|proxy_wallet_address|user_id|preferences_id`

---

## Русский

### Установка

#### Предварительные требования

- **Rust** : Убедитесь, что Rust установлен. Вы можете скачать и установить Rust с [https://www.rust-lang.org/tools/install]().

### Сборка

Клонируйте репозиторий и соберите проект:

```
git clone https://github.com/Fragment-Software/polymarket-eye.git
cd polymarket-eye
cargo build --release
```

### Конфигурация

Перед запуском программного обеспечения настройте необходимые файлы:

1. **private_keys.txt** : Добавьте ваши приватные ключи в `data/private_keys.txt`.
2. **proxies.txt** : Добавьте ваши прокси в `data/proxies.txt`.

### Запуск

Запустите собранный бинарный файл:

`cargo run --release `

### Вывод

После запуска результат будет сохранен в `data/out.txt` в следующем формате:

`wallet_address|proxy_wallet_address|user_id|preferences_id`
