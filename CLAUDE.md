# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TTS Bot is a Discord text-to-speech bot written in Rust, using Serenity (Discord API), Songbird (voice), and Poise (command framework). The bot reads messages from text channels and speaks them in voice channels using various TTS engines (gTTS, eSpeak, Google Cloud, AWS Polly).

## Build and Development Commands

### Building
```bash
cargo build              # Debug build
cargo build --release    # Production build
```

### Running
```bash
cargo run                # Run in debug mode
cargo run --release      # Run optimized binary
```

### Linting
```bash
cargo clippy             # Run clippy with project-specific lints
```

Note: This project requires Rust nightly (1.88+) as specified in Cargo.toml. The `edition = "2024"` field is used.

### Dependencies
- PostgreSQL database (required)
- FFmpeg (for audio processing)
- External TTS service (see `tts_service` in config)

## Configuration

Copy `config-selfhost.toml` to `config.toml` and fill in required fields:
- Discord bot token
- PostgreSQL connection details
- TTS service URL and auth key
- Webhook URLs for logging
- Main server and channel IDs

## Architecture

### Workspace Structure

This is a Cargo workspace with 5 crates:

1. **tts_core**: Core types, database handlers, error handling, and shared utilities
   - `structs.rs`: Main `Data` struct, config types, enums like `TTSMode`
   - `database.rs`: Database abstraction layer using `create_db_handler!` macro
   - `common.rs`: Shared functions for audio fetching, URL preparation, premium checks
   - `errors.rs`: Centralized error handling

2. **tts_commands**: Discord command definitions using Poise
   - `main_.rs`: Core commands (join, leave, setup)
   - `settings/`: Configuration commands with voice selection UI
   - `premium.rs`: Premium subscription commands
   - `owner.rs`: Bot owner-only commands
   - Command registration via `commands()` function

3. **tts_events**: Discord event handlers (Serenity EventHandler implementation)
   - `message/tts.rs`: Core TTS message processing logic
   - `voice_state.rs`: Voice channel state tracking
   - `guild.rs`, `member.rs`, `channel.rs`: Lifecycle event handlers
   - `ready.rs`: Bot startup and shard management

4. **tts_tasks**: Background tasks and logging
   - Webhook-based logging infrastructure
   - Analytics collection
   - Periodic maintenance tasks (implements `Looper` trait)

5. **tts_migrations**: Database schema and migrations
   - `load_db_and_conf()`: Loads config and initializes database pool

### Key Design Patterns

**Database Handlers**: The `create_db_handler!` macro creates type-safe database accessors. Used for:
- `guilds_db`: Guild-level settings (prefix, premium status, voice mode)
- `userinfo_db`: User preferences
- `user_voice_db` / `guild_voice_db`: Voice selection per TTS mode
- `nickname_db`: Custom pronunciation nicknames

**Data Structure**: The main `Data` struct (Arc-wrapped) contains:
- All database handlers
- Voice lists for each TTS mode (gTTS, eSpeak, Google Cloud, Polly)
- HTTP client, Songbird instance, analytics handler
- Caches: `entitlement_cache`, `join_vc_tokens`, `last_to_xsaid_tracker`
- Config objects

**Event Flow**:
1. Message arrives → `tts_events::message::handle()`
2. Check if channel is setup for TTS → fetch guild settings from `guilds_db`
3. Process message content (regex filtering via `RegexCache`)
4. Prepare TTS request → `tts_core::common::prepare_url()`
5. Fetch audio from TTS service → `fetch_audio()`
6. Play via Songbird in voice channel

**Premium System**: Uses Discord's native monetization (SKUs) plus optional Patreon integration. Premium checks occur via `Data::premium_check()` before premium commands execute.

### Important Patterns

- **Error Handling**: All errors flow through `tts_core::errors::handle()` and variants. Never panic in async event handlers.
- **Voice Channel Locking**: `JoinVCToken::acquire()` prevents race conditions when joining voice channels.
- **Fixed-Size Strings**: Uses `FixedString` and `ArrayString` to avoid heap allocations for small strings.
- **Disallowed Methods**: See `clippy.toml` - certain APIs like `Option::map_or` and `Songbird::remove` are banned in favor of safer alternatives.

## Testing

There are no automated tests in this repository. Testing is done manually:
1. Run the bot with a test configuration
2. Join a voice channel and run commands in a setup text channel
3. Verify TTS audio plays correctly

## Database Schema

The bot uses PostgreSQL with the following key tables (managed via `tts_migrations`):
- `guilds`: Per-guild settings
- `userinfo`: Per-user preferences
- `user_voice` / `guild_voice`: Voice selections by mode
- `nicknames`: Custom pronunciation mappings

Use `sqlx::query!` and `sqlx::query_as!` macros for compile-time verified queries.

## External Service Integration

The bot requires a separate TTS service (see `config.main.tts_service`). It expects these endpoints:
- `GET /voices?mode={mode}&raw=true`: Returns available voices for a TTS mode
- `GET /translation_languages`: Returns supported translation languages
- `GET /speak?text=...&lang=...&mode=...`: Returns audio stream

Authentication via `Authorization` header using `tts_service_auth_key`.

## Common Issues

- **Voice not working**: Ensure FFmpeg is installed and in PATH
- **Database connection errors**: Check PostgreSQL connection string in config
- **TTS service errors**: Verify `tts_service` URL and auth key are correct
- **Clippy warnings**: The project has extensive clippy lints enabled at workspace level
