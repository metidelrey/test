from PyQt6.QtWidgets import (

    QPushButton
)
import random

class ChipButton(QPushButton):
    def __init__(self, text, parent=None):
        super().__init__(text, parent)
        background_color = "#0069d9"
        self.setStyleSheet("""
            QPushButton {
                background-color: #e0e0e0;
                border: none;
                color: #f1f1f1;
                border-radius: 10px; /* Rounded edges */
                padding: 4px 6px; /* Padding around text */
                font-size: 12px;
                background-color:%s;
            }
            QPushButton:hover {
                background-color: #b0b0b0; /* Change color on hover */
            }
            QPushButton:pressed {
                background-color: #a0a0a0; /* Change color on press */
            }
        """ % (background_color))
        self.setFixedWidth(60)
        