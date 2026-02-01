use super::*;

#[test]
fn test_campaign_genre_from_system() {
    assert_eq!(CampaignGenre::from_system("D&D 5e"), CampaignGenre::Fantasy);
    assert_eq!(CampaignGenre::from_system("Pathfinder 2e"), CampaignGenre::Fantasy);
    assert_eq!(CampaignGenre::from_system("Call of Cthulhu"), CampaignGenre::Horror);
    assert_eq!(CampaignGenre::from_system("Vampire: The Masquerade"), CampaignGenre::Horror);
    assert_eq!(CampaignGenre::from_system("Cyberpunk Red"), CampaignGenre::Cyberpunk);
    assert_eq!(CampaignGenre::from_system("Shadowrun"), CampaignGenre::Cyberpunk);
    assert_eq!(CampaignGenre::from_system("Traveller"), CampaignGenre::SciFi);
    assert_eq!(CampaignGenre::from_system("Alien RPG"), CampaignGenre::SciFi);
    assert_eq!(CampaignGenre::from_system("Modern AGE"), CampaignGenre::Modern);
    assert_eq!(CampaignGenre::from_system("Pendragon"), CampaignGenre::Historical);
    assert_eq!(CampaignGenre::from_system("Unknown System"), CampaignGenre::Unknown);
}
