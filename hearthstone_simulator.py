from game.game_controller import GameController
from game.game_model import GameModel
from game.game_view import GameView

import sys
from PyQt5.QtWidgets import QApplication, QMainWindow


class HearthstoneSimulator(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Hearthstone Simulator")
        self.window().showMaximized()

        self.model = GameModel()
        self.controller = GameController(
            self.model, None)  # View is not created yet
        self.view = GameView(self.controller)

        self.controller.view = self.view  # Update the controller with the view instance
        self.setCentralWidget(self.view)


if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = HearthstoneSimulator()
    window.show()
    sys.exit(app.exec_())