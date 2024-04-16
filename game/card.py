class Card:
    def __init__(self, ingame_id: int, card_id: int, name: str, cost: int, attack: int, health: int, image_path: str):
        self.ingame_id = ingame_id  # Unique identifier for instances of cards in the game
        self.card_id = card_id  # Unique identifier for the card type
        self.name = name  # The card's name
        self.cost = cost  # Mana cost to play the card
        self.attack = attack  # The card's attack value
        self.health = health  # The card's health value
        self.image_path = image_path

    def __str__(self):
        return f"{self.name} (Cost: {self.cost}, Attack: {self.attack}, Health: {self.health})"

    def take_damage(self, damage: int):
        """
        Apply damage to the card, reducing its health.
        """
        self.health -= damage
        if self.health <= 0:
            self.on_death()

    def on_death(self):
        """
        Handle the card's death (when health <= 0).
        This method can be overridden in subclasses for cards with special effects.
        """
        print(f"{self.name} has been destroyed.")