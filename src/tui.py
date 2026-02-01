#!/usr/bin/env python3
"""
MotherDuck Explain Chart TUI

A terminal UI for viewing chart data with data lineage.
Watches ~/.claude/explain-chart/current.json for updates.
"""

import sys
import time
from typing import Optional

from rich.console import Console
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.text import Text

from .views import (
    render_waiting_state,
    render_query_view,
    render_mask_view,
    render_data_view,
    render_chart_view,
)
from .watcher import watch_file, read_data_file


class TUIApp:
    """Main TUI application."""

    TAB_NAMES = ["Query", "Mask", "Data", "Chart"]

    def __init__(self):
        self.console = Console()
        self.data: Optional[dict] = None
        self.active_tab = 0
        self.scroll_offset = 0
        self.selected_point = 0  # Index of selected data point in Chart view
        self.frame = 0
        self.running = True
        self.observer = None

    def on_data_update(self, new_data: dict) -> None:
        """Handle data file updates."""
        self.data = self._process_data(new_data)
        self.active_tab = 0
        self.scroll_offset = 0
        self.selected_point = 0  # Reset selection on new data

    def _process_data(self, data: dict) -> dict:
        """Process data, converting columnar format to objects if needed."""
        rows = data.get("rows", [])
        columns = data.get("columns", [])

        # If rows are arrays (columnar format), convert to dicts
        if rows and isinstance(rows[0], list):
            rows = [dict(zip(columns, row)) for row in rows]

        return {
            **data,
            "rows": rows,
        }

    def render_header(self, title: Optional[str] = None) -> Text:
        """Render the header with MotherDuck branding."""
        header = Text()
        header.append("<(o)>", style="yellow bold")
        header.append(" MotherDuck Explain Chart", style="white bold")
        if title:
            header.append(f" - {title}", style="dim")
        return header

    def render_tab_bar(self) -> Text:
        """Render the tab bar."""
        tab_bar = Text()
        for i, name in enumerate(self.TAB_NAMES):
            if i == self.active_tab:
                tab_bar.append(f"[{name}]", style="bold cyan reverse")
            else:
                tab_bar.append(f" {name} ", style="dim")
            if i < len(self.TAB_NAMES) - 1:
                tab_bar.append(" - ")
        return tab_bar

    def render_footer(self) -> Text:
        """Render the footer with keyboard shortcuts."""
        footer = Text()
        footer.append("<- ->", style="cyan")
        footer.append(" tabs  ", style="dim")
        if self.active_tab == 2:  # Data view
            footer.append("↑↓", style="cyan")
            footer.append(" select row  ", style="dim")
        elif self.active_tab == 3:  # Chart view
            footer.append("↑↓", style="cyan")
            footer.append(" select point  ", style="dim")
        else:
            footer.append("↑↓", style="cyan")
            footer.append(" scroll  ", style="dim")
        footer.append("1-4", style="cyan")
        footer.append(" jump  ", style="dim")
        footer.append("q", style="cyan")
        footer.append(" quit  ", style="dim")
        footer.append("<(o)>", style="yellow")
        return footer

    def render_content(self) -> Panel:
        """Render the current view content."""
        if not self.data:
            return render_waiting_state(self.frame)

        title = self.data.get("title", "")
        query = self.data.get("query", "")
        x_field = self.data.get("xField", "")
        y_field = self.data.get("yField", "")
        columns = self.data.get("columns", [])
        rows = self.data.get("rows", [])
        chart_type = self.data.get("chartType")

        # Get terminal size for dynamic sizing
        width = self.console.width
        height = self.console.height
        visible_rows = max(4, min(height - 12, 15))
        chart_height = max(10, min(height - 14, 16))
        chart_width = max(30, width - 25)

        # Use consistent content height across all tabs (based on chart size)
        content_height = chart_height + 6  # Chart + title + labels + detail line

        if self.active_tab == 0:
            return render_query_view(query, content_height)
        elif self.active_tab == 1:
            return render_mask_view(x_field, y_field, columns, content_height)
        elif self.active_tab == 2:
            return render_data_view(rows, columns, x_field, y_field, self.scroll_offset, visible_rows, self.selected_point, content_height)
        else:
            return render_chart_view(rows, title, x_field, y_field, chart_type, chart_width, chart_height, self.selected_point)

    def render(self) -> Panel:
        """Render the full TUI."""
        layout = Layout()

        # Build content
        header = self.render_header(self.data.get("title") if self.data else None)
        tab_bar = self.render_tab_bar() if self.data else Text()
        content = self.render_content()
        footer = self.render_footer()

        # Combine into a panel
        from rich.console import Group

        inner = Group(
            header,
            Text(),
            tab_bar,
            Text(),
            content,
            Text(),
            footer,
        )

        return Panel(
            inner,
            border_style="yellow",
            padding=(0, 1),
        )

    def handle_key(self, key: str) -> None:
        """Handle keyboard input."""
        if key == "q":
            self.running = False
        elif key == "left":
            self.active_tab = max(0, self.active_tab - 1)
            self.scroll_offset = 0
        elif key == "right":
            self.active_tab = min(3, self.active_tab + 1)
            self.scroll_offset = 0
        elif key in "1234":
            self.active_tab = int(key) - 1
            self.scroll_offset = 0
        elif key == "up":
            if self.active_tab in (2, 3) and self.data:  # Data or Chart view
                rows = self.data.get("rows", [])
                if rows:
                    self.selected_point = (self.selected_point - 1) % len(rows)
                    # Auto-scroll to keep selection visible (for Data tab)
                    if self.active_tab == 2:
                        visible_rows = max(4, min(self.console.height - 12, 15))
                        if self.selected_point < self.scroll_offset:
                            self.scroll_offset = self.selected_point
                        elif self.selected_point >= self.scroll_offset + visible_rows:
                            self.scroll_offset = self.selected_point - visible_rows + 1
            else:
                self.scroll_offset = max(0, self.scroll_offset - 1)
        elif key == "down":
            if self.active_tab in (2, 3) and self.data:  # Data or Chart view
                rows = self.data.get("rows", [])
                if rows:
                    self.selected_point = (self.selected_point + 1) % len(rows)
                    # Auto-scroll to keep selection visible (for Data tab)
                    if self.active_tab == 2:
                        visible_rows = max(4, min(self.console.height - 12, 15))
                        if self.selected_point < self.scroll_offset:
                            self.scroll_offset = self.selected_point
                        elif self.selected_point >= self.scroll_offset + visible_rows:
                            self.scroll_offset = self.selected_point - visible_rows + 1
            elif self.data:
                rows = self.data.get("rows", [])
                visible_rows = max(4, min(self.console.height - 12, 15))
                max_scroll = max(0, len(rows) - visible_rows)
                self.scroll_offset = min(max_scroll, self.scroll_offset + 1)

    def _read_key(self, fd) -> str:
        """Read a key or escape sequence from stdin."""
        import os
        import fcntl

        c = os.read(fd, 1).decode('utf-8', errors='ignore')

        if c == "\x1b":  # Escape sequence - try to read more
            # Temporarily set non-blocking
            flags = fcntl.fcntl(fd, fcntl.F_GETFL)
            fcntl.fcntl(fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)
            try:
                seq = os.read(fd, 2).decode('utf-8', errors='ignore')
                if seq == "[A":
                    return "up"
                elif seq == "[B":
                    return "down"
                elif seq == "[C":
                    return "right"
                elif seq == "[D":
                    return "left"
            except BlockingIOError:
                pass
            finally:
                fcntl.fcntl(fd, fcntl.F_SETFL, flags)
            return "escape"

        return c

    def run(self) -> None:
        """Run the TUI application."""
        import select
        import termios
        import tty

        # Start file watcher
        self.observer = watch_file(self.on_data_update)

        # Set up terminal for raw input
        fd = sys.stdin.fileno()
        old_settings = termios.tcgetattr(fd)

        try:
            tty.setcbreak(fd)

            with Live(self.render(), console=self.console, refresh_per_second=4, screen=True) as live:
                while self.running:
                    # Check for keyboard input (non-blocking)
                    if select.select([fd], [], [], 0.1)[0]:
                        key = self._read_key(fd)
                        self.handle_key(key)

                    # Update animation frame
                    self.frame += 1

                    # Re-render
                    live.update(self.render())

        finally:
            # Restore terminal settings
            termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)

            # Stop file watcher
            if self.observer:
                self.observer.stop()
                self.observer.join()


def main():
    """Entry point for the TUI."""
    app = TUIApp()
    try:
        app.run()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
