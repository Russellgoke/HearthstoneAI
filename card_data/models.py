from sqlalchemy import  Column, String, Integer
from card_data.database import Base

class CardModel(Base):
    __tablename__ = 'cards'
    id = Column(Integer, primary_key=True)
    name = Column(String, unique=True, nullable=False)
    image_path = Column(String)
    type = Column(String)
    cost = Column(Integer)
    attack = Column(Integer)
    health = Column(Integer)
    card_text = Column(String)
