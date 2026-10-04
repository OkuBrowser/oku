use connexa::builder::IntoKeypair;
use connexa::keystore::Keychain;
use connexa::keystore::generate_key;
use connexa::keystore::store::memory::MemoryKeystore;
use libp2p::identity::Keypair;

#[test]
fn test_keypair_into_keypair() {
    let original = Keypair::generate_ed25519();
    let original_public = original.public();

    let result = original
        .clone()
        .into_keypair()
        .expect("should convert successfully");
    assert_eq!(result.public(), original_public);
}

#[test]
fn test_keypair_ref_into_keypair() {
    let original = Keypair::generate_ed25519();
    let original_public = original.public();

    let result = (&original)
        .into_keypair()
        .expect("should convert successfully");
    assert_eq!(result.public(), original_public);

    // Verify original is still usable
    assert_eq!(original.public(), original_public);
}

#[test]
#[cfg(feature = "testing")]
fn test_u8_into_keypair() {
    let seed: u8 = 42;
    let keypair1 = seed.into_keypair().expect("should convert successfully");
    let keypair2 = seed.into_keypair().expect("should convert successfully");

    // Same seed should produce same keypair
    assert_eq!(keypair1.public(), keypair2.public());

    // Different seeds should produce different keypairs
    let keypair3 = 43u8.into_keypair().expect("should convert successfully");
    assert_ne!(keypair1.public(), keypair3.public());
}

#[test]
fn test_vec_u8_into_keypair() {
    let bytes = vec![1u8; 32]; // Ed25519 requires exactly 32 bytes
    let keypair = bytes
        .clone()
        .into_keypair()
        .expect("should convert successfully");

    // Same bytes should produce same keypair
    let keypair2 = bytes.into_keypair().expect("should convert successfully");
    assert_eq!(keypair.public(), keypair2.public());
}

#[test]
fn test_vec_u8_invalid_length() {
    let bytes = vec![1u8; 31]; // Invalid length for Ed25519
    assert!(
        bytes.into_keypair().is_err(),
        "should fail with invalid length"
    );

    let bytes = vec![1u8; 33]; // Invalid length for Ed25519
    assert!(
        bytes.into_keypair().is_err(),
        "should fail with invalid length"
    );
}

#[test]
fn test_slice_into_keypair() {
    let mut bytes = [0u8; 32];
    bytes[0] = 42;

    let keypair = (&mut bytes[..])
        .into_keypair()
        .expect("should convert successfully");

    // Create a fresh copy of the same bytes for comparison
    let mut bytes2 = [0u8; 32];
    bytes2[0] = 42;
    let keypair2 = (&mut bytes2[..])
        .into_keypair()
        .expect("should convert successfully");
    assert_eq!(keypair.public(), keypair2.public());
}

#[test]
fn test_slice_invalid_length() {
    let mut bytes = [0u8; 31];
    assert!(
        (&mut bytes[..]).into_keypair().is_err(),
        "should fail with invalid length"
    );
}

#[test]
fn test_option_some_into_keypair() {
    let original = Keypair::generate_ed25519();
    let original_public = original.public();

    let option: Option<Keypair> = Some(original);
    let result = option.into_keypair().expect("should convert successfully");
    assert_eq!(result.public(), original_public);
}

#[test]
fn test_option_none_into_keypair() {
    let option: Option<Keypair> = None;
    let result = option.into_keypair().expect("should generate new keypair");

    // Each None should generate a different keypair
    let option2: Option<Keypair> = None;
    let result2 = option2.into_keypair().expect("should generate new keypair");
    assert_ne!(result.public(), result2.public());
}

#[test]
#[cfg(feature = "testing")]
fn test_option_u8_into_keypair() {
    // Test Option<u8> with Some
    let option: Option<u8> = Some(42);
    let result = option.into_keypair().expect("should convert successfully");

    // Should produce same keypair as direct u8 conversion
    let direct = 42u8.into_keypair().expect("should convert successfully");
    assert_eq!(result.public(), direct.public());

    // Test Option<u8> with None
    let option: Option<u8> = None;
    let result = option.into_keypair().expect("should generate new keypair");
    assert_ne!(result.public(), direct.public());
}

#[test]
fn test_option_vec_into_keypair() {
    // Test Option<Vec<u8>> with Some
    let bytes = vec![1u8; 32];
    let option: Option<Vec<u8>> = Some(bytes.clone());
    let result = option.into_keypair().expect("should convert successfully");

    // Should produce same keypair as direct Vec conversion
    let direct = bytes.into_keypair().expect("should convert successfully");
    assert_eq!(result.public(), direct.public());

    // Test Option<Vec<u8>> with None
    let option: Option<Vec<u8>> = None;
    let result = option.into_keypair().expect("should generate new keypair");
    assert_ne!(result.public(), direct.public());
}

#[test]
fn test_deterministic_keypair_generation() {
    // Test that the same bytes always produce the same keypair
    let mut bytes1 = [0u8; 32];
    bytes1[0] = 1;
    bytes1[31] = 255;

    let mut bytes2 = bytes1;

    let kp1 = (&mut bytes1[..]).into_keypair().expect("should convert");
    let kp2 = (&mut bytes2[..]).into_keypair().expect("should convert");

    assert_eq!(kp1.public(), kp2.public());
}

#[test]
fn test_different_bytes_produce_different_keypairs() {
    let bytes1 = vec![1u8; 32];
    let mut bytes2 = vec![1u8; 32];
    bytes2[0] = 2;

    let kp1 = bytes1.into_keypair().expect("should convert");
    let kp2 = bytes2.into_keypair().expect("should convert");

    assert_ne!(kp1.public(), kp2.public());
}

#[test]
fn test_keypair_type_preservation() {
    // Ensure Ed25519 keypairs are created correctly
    let bytes = vec![42u8; 32];
    let keypair = bytes.into_keypair().expect("should convert");

    // The keypair should be Ed25519; verify the public key converts to a PeerId
    let peer_id = keypair.public().to_peer_id();
    assert!(!peer_id.to_string().is_empty());
}

#[test]
#[cfg(feature = "testing")]
fn test_u8_seed_consistency() {
    // Test that u8 seed creates consistent keypairs
    for seed in 0u8..10 {
        let kp1 = seed.into_keypair().expect("should convert");
        let kp2 = seed.into_keypair().expect("should convert");
        assert_eq!(
            kp1.public(),
            kp2.public(),
            "Same seed {seed} should produce same keypair",
        );
    }
}

#[test]
fn test_with_existing_identity_builder() {
    use connexa::prelude::DefaultConnexaBuilder;

    // Test with Keypair
    let kp = Keypair::generate_ed25519();
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(kp.clone()).expect("should create builder");

    // Test with &Keypair
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(&kp).expect("should create builder");

    // Test with Vec<u8>
    let bytes = vec![1u8; 32];
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(bytes).expect("should create builder");

    // Test with Option<Keypair>
    let option: Option<Keypair> = Some(kp);
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(option).expect("should create builder");

    // Test with None
    let option: Option<Keypair> = None;
    let _builder = DefaultConnexaBuilder::with_existing_identity(option)
        .expect("should create builder with generated keypair");
}

#[test]
#[cfg(feature = "testing")]
fn test_with_existing_identity_builder_u8() {
    use connexa::prelude::DefaultConnexaBuilder;

    // Test with u8 seed
    let seed: u8 = 42;
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(seed).expect("should create builder");

    // Test with Option<u8>
    let option: Option<u8> = Some(42);
    let _builder =
        DefaultConnexaBuilder::with_existing_identity(option).expect("should create builder");
}

#[test]
fn test_with_identity_and_keychain_builder() {
    use connexa::prelude::DefaultConnexaBuilder;

    let kp = Keypair::generate_ed25519();
    let keychain: Keychain<MemoryKeystore> = Keychain::new(generate_key());
    let _builder = DefaultConnexaBuilder::with_identity_and_keychain(kp, keychain)
        .expect("should create builder");
}

#[tokio::test]
async fn test_new_identity_builds_ephemeral() {
    use connexa::prelude::DefaultConnexaBuilder;

    let a = DefaultConnexaBuilder::new_identity()
        .enable_memory_transport()
        .build()
        .await
        .expect("new_identity should build with an ephemeral keypair");
    let b = DefaultConnexaBuilder::new_identity()
        .enable_memory_transport()
        .build()
        .await
        .expect("new_identity should build with an ephemeral keypair");

    // With no keychain enabled the identity is not persisted, so each build differs.
    assert_ne!(
        a.keypair().public().to_peer_id(),
        b.keypair().public().to_peer_id()
    );
}
