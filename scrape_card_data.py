# add_cards.py
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from card_data.models import CardModel

engine = create_engine('sqlite:///card_data/cards.db')
Session = sessionmaker(bind=engine)
session = Session()
session.query(CardModel).delete()


# Create some card instances
card1 = CardModel(id=0, name="Murloc Raider", cost=1, attack=2, health=1,
                  image_path="/Murloc_Raider.png")
card2 = CardModel(id=1, name="River Crocolisk", cost=2, attack=2, health=3,
                  image_path="/River_Crocolisk.png")

# Add cards to the session and commit to the database
session.add(card1)
session.add(card2)
session.commit()
