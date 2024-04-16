from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from card_data.models import Base, CardModel  # Adjust the import path as needed
from game.card import Card  # Adjust the import path as needed

class GameController:
    def __init__(self, model, view):
        self.model = model
        self.view = view
        self.model.init_decks(self.create_deck(), self.create_deck())
    
    def draw_card(self) -> None:
        self.model.draw_card()
        self.update_view()
    
    def update_view(self) -> None:
        self.view.update(self.model)

    def create_deck(self):
        # Setup SQLAlchemy session
        engine = create_engine('sqlite:///card_data/cards.db')
        Session = sessionmaker(bind=engine)
        session = Session()

        # Load card data from the database
        db_cards = session.query(CardModel).all()

        # Create Card instances for the game
        game_cards = [Card(ingame_id=i, card_id=card.id, name=card.name, cost=card.cost, attack=card.attack, health=card.health, image_path=card.image_path) for i, card in enumerate(db_cards)]
        # Don't forget to close the session when done
        session.close()
        return game_cards
