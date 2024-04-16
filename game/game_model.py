class GameModel:
    def __init__(self):
        self.deck = []
        self.hand = []
        self.board = []
        self.opp_deck = []
        self.opp_hand = []
        self.opp_board = []

    def init_decks(self, deck, opp_deck):
        self.deck = deck
        self.opp_deck = opp_deck

    def draw_card(self):
        if self.deck:
            self.hand.append(self.deck.pop(0))
        else:
            pass # todo fatigue

    def play_card_from_hand(self, card_index):
        if card_index < len(self.hand):
            self.board.append(self.hand.pop(card_index))
