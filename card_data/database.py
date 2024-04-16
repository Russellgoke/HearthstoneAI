from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, declarative_base

# Define the SQLite database URL
# Adjust the path as necessary, e.g., if you're accessing it from a different directory level
DATABASE_URL = "sqlite:///card_data/cards.db"

# Create the SQLAlchemy engine
engine = create_engine(DATABASE_URL, echo=True)

# Create a configured "Session" class
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Base class for declarative class definitions
Base = declarative_base()

def init_db():
    # Import all modules here that might define models so that
    # they will be registered properly on the metadata. Otherwise
    # SQLAlchemy might have issues creating the tables properly.
    from card_data.models import CardModel  # Assuming your models are defined in card_data/models.py
    Base.metadata.create_all(bind=engine)
