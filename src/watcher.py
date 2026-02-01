"""File watcher for live updates from Claude Code."""

import json
import os
import time
from pathlib import Path
from typing import Callable, Optional
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler, FileModifiedEvent, FileCreatedEvent


DATA_FILE_PATH = Path.home() / ".claude" / "explain-chart" / "current.json"


def ensure_data_dir() -> None:
    """Ensure the data directory exists."""
    DATA_FILE_PATH.parent.mkdir(parents=True, exist_ok=True)


def read_data_file() -> Optional[dict]:
    """Read and parse the data file."""
    try:
        if DATA_FILE_PATH.exists():
            content = DATA_FILE_PATH.read_text()
            return json.loads(content)
    except (json.JSONDecodeError, IOError):
        pass
    return None


class DataFileHandler(FileSystemEventHandler):
    """Handler for file system events on the data file."""

    def __init__(self, callback: Callable[[dict], None], debounce_ms: int = 100):
        self.callback = callback
        self.debounce_ms = debounce_ms
        self._last_modified = 0

    def _handle_change(self) -> None:
        """Handle file change with debouncing."""
        now = time.time() * 1000
        if now - self._last_modified < self.debounce_ms:
            return
        self._last_modified = now

        data = read_data_file()
        if data:
            self.callback(data)

    def on_modified(self, event) -> None:
        if isinstance(event, FileModifiedEvent) and event.src_path == str(
            DATA_FILE_PATH
        ):
            self._handle_change()

    def on_created(self, event) -> None:
        if isinstance(event, FileCreatedEvent) and event.src_path == str(
            DATA_FILE_PATH
        ):
            self._handle_change()


def watch_file(callback: Callable[[dict], None]) -> Observer:
    """Watch the data file for changes and call callback on updates.

    Returns the observer instance which can be stopped with observer.stop().
    """
    ensure_data_dir()

    # Read initial data if it exists
    initial_data = read_data_file()
    if initial_data:
        callback(initial_data)

    # Set up file watcher
    handler = DataFileHandler(callback)
    observer = Observer()
    observer.schedule(handler, str(DATA_FILE_PATH.parent), recursive=False)
    observer.start()

    return observer
