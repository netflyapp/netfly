# Netfly — integracja haseł przez macOS Keychain

## Cel
Lekkie uzupełnianie haseł w Netfly przez systemowy Keychain (zero zewnętrznych zależności).

## Architektura
```
login form → userscript detectuje input[type=password]
    → IPC do Rust (tauri invoke)
        → Rust: security find-internet-password -s <domain> -w
            → macOS Keychain → zwraca hasło
    ← Rust zwraca JSON { username, password }
→ userscript wypełnia formularz
```

## Kroki

### 1. Rust: dodaj moduł `keychain.rs`
- Plik: `src-tauri/src/keychain.rs`
- Funkcja `get_password(domain: &str) -> Result<Option<LoginEntry>, String>` — woła `security find-internet-password` przez `std::process::Command`
- Funkcja `save_password(domain: &str, username: &str, password: &str) -> Result<()>` — woła `security add-internet-password`
- Struktura `LoginEntry { username, password }`
- **Zero nowych crate'ów** — tylko `std::process::Command`

### 2. Rust: dodaj command Tauri
- W `lib.rs`: `#[tauri::command] fn keychain_get(domain: String) -> Option<LoginEntry>`
- Rejestruj w `generate_handler![]`
- Opcjonalnie: `#[tauri::command] fn keychain_save(domain, username, password)`

### 3. Frontend: IPC wrapper
- W `src/ipc.ts` dodaj:
  - `ipc.keychainGet(domain: string): Promise<LoginEntry | null>`
  - `ipc.keychainSave(domain, username, password): Promise<void>`

### 4. Userscript `autofill.user.js`
- `@match *://*/*`, `@run-at document-end`
- Obserwuje `input[type="password"]` focus / form submit
- Na focus: wyciąga domenę z `location.hostname`, woła `window.__TAURI__.invoke('keychain_get', { domain })`, wypełnia `input[type="text"]` / `input[type="email"]` obok
- Na form submit: pyta "Save password?" → keychain_save

### 5. Opcjonalnie: import z CSV
- Netfly palette command lub oddzielny skrypt do importu CSV (Bitwarden/Chrome export) → batch save do Keychain

### 6. Test
- `cargo build`
- Otwórz stronę z loginem — sprawdź czy autofill działa
- Sprawdź Keychain.app → czy hasła są zapisane

## Uwagi
- `security` CLI jest już w systemie — **0 nowych zależności**, binarka ~0 bytes
- Keychain jest szyfrowany systemowo (master password to hasło do konta macOS)
- Działa tylko na macOS — celowo, Netfly jest macOS-only
- Pierwsze wywołanie może pokazać systemowy prompt "Netfly wants to access Keychain"
