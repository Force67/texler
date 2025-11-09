# Texler

Collaborative LaTeX IDE with live preview and Docker-based compilation.

## Features
- Real-time collaborative editing
- Live PDF preview with auto-compile
- Modern Monaco editor with LaTeX syntax highlighting
- Self-hosted with Docker containers
- Project management and version control

## Architecture
- **Frontend**: React + TypeScript + Monaco Editor
- **Backend**: Node.js + Express + WebSocket
- **LaTeX**: TeXLive in Docker container
- **Storage**: PostgreSQL + Redis + MinIO

## Quick Start
```bash
docker-compose up
```

## Development
```bash
cd apps/web
npm install
npm run dev
```