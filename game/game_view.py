from PyQt5.QtWidgets import QVBoxLayout, QHBoxLayout, QVBoxLayout, QPushButton, QLabel, QWidget, QLabel, QApplication
from PyQt5.QtCore import Qt, QSize
from PyQt5.QtGui import QResizeEvent, QPixmap

from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from game.game_controller import GameController

from game.card import Card


class GameView(QWidget):
    def __init__(self, controller: 'GameController') -> None:
        super().__init__()
        self.controller = controller
        self.initUI()

    def initUI(self) -> None:
        # Example window size, adjust as needed
        mainLayout = QVBoxLayout(self)
        self.setLayout(mainLayout)

        # Opponent's layouts
        self.opponentBoardLayout = QHBoxLayout()
        self.opponentBoardLayout.setAlignment(Qt.AlignCenter)
        mainLayout.addLayout(self.opponentBoardLayout)

        self.opponentHandLayout = QHBoxLayout()
        self.opponentHandLayout.setAlignment(Qt.AlignCenter)
        mainLayout.addLayout(self.opponentHandLayout)

        mainLayout.addStretch(1)

        # Player's layouts
        self.boardLayout = QHBoxLayout()
        self.boardLayout.setAlignment(Qt.AlignCenter)
        mainLayout.addLayout(self.boardLayout)

        self.handLayout = QHBoxLayout()
        self.handLayout.setAlignment(Qt.AlignCenter)
        mainLayout.addLayout(self.handLayout)

        # Draw button that stays centered vertically within the window
        self.btn_draw = QPushButton('Draw Card', self)
        self.btn_draw.clicked.connect(self.controller.draw_card)

    def resizeEvent(self, event: QResizeEvent) -> None:
        super().resizeEvent(event)
        # Keep the button centered vertically and to the right
        btnSize = self.btn_draw.sizeHint()
        self.btn_draw.move(self.width() - btnSize.width() -
                           20, (self.height() - btnSize.height()) // 2)

    def create_card_widget(self, card: Card) -> QLabel:
        card_widget = QWidget()
        layout = QVBoxLayout()

        # QLabel for the image
        image_label = QLabel()
        image_path = './card_data/card_images' + card.image_path
        pixmap = QPixmap(image_path)
        fixed_size = QSize(100, 150)  # Desired fixed size for cards
        scaled_pixmap = pixmap.scaled(
            fixed_size, Qt.KeepAspectRatio, Qt.SmoothTransformation)
        image_label.setPixmap(scaled_pixmap)
        image_label.setFixedSize(fixed_size)

        # Overlay stats on the image
        # This is a simplistic approach; for more complex overlays, consider a custom paint event

        # Add the image and stats labels to the layout
        layout.addWidget(image_label)
        card_widget.setLayout(layout)

        return card_widget

    def update(self, model) -> None:
        # Clear existing cards for all layouts
        for layout in [self.handLayout, self.boardLayout, self.opponentHandLayout, self.opponentBoardLayout]:
            for i in reversed(range(layout.count())):
                layout.itemAt(i).widget().setParent(None)

        # Update player's hand cards
        for card in model.hand:
            card_widget = self.create_card_widget(card)
            self.handLayout.addWidget(card_widget)

        # Update player's board cards
        for card in model.board:
            card_widget = self.create_card_widget(card)
            self.boardLayout.addWidget(card_widget)

        # Update opponent's hand cards
        for card in model.opp_hand:
            card_widget = self.create_card_widget(card)
            self.opponentHandLayout.addWidget(card_widget)

        # Update opponent's board cards
        for card in model.opp_board:
            card_widget = self.create_card_widget(card)
            self.opponentBoardLayout.addWidget(card_widget)
