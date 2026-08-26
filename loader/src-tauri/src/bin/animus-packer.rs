use base64::{
    engine::general_purpose::STANDARD as BASE64,
    Engine as _,
};

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};

use chrono::Utc;

use ed25519_dalek::{
    Signer,
    SigningKey,
};

use rand::{
    rngs::OsRng,
    RngCore,
};

use serde_json::json;

use sha2::{
    Digest,
    Sha256,
};

use std::{
    env,
    error::Error,
    fs::{
        self,
        File,
        OpenOptions,
    },
    io::{
        self,
        Read,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
    process,
};

use uuid::Uuid;

use zeroize::Zeroize;


// ============================================================
// ANIMUS PACKAGE FORMAT v1
// ============================================================

const MAGIC: &[u8; 8] = b"ANIMUS01";

const FOOTER_MAGIC: &[u8; 8] = b"ANMSIG01";

const FORMAT_VERSION: u16 = 1;

/// Büyük dosyalar RAM'e alınmaz.
/// Her seferinde 4 MB şifrelenir.
const CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// XChaCha20-Poly1305 authentication tag boyutu.
const TAG_SIZE: usize = 16;

/// İmza domain separation.
const SIGN_DOMAIN: &[u8] =
    b"ANIMUS-PACKAGE-SIGNATURE-V1\0";


// ============================================================
// MAIN
// ============================================================

fn main() {
    if let Err(error) = run() {
        eprintln!();
        eprintln!("========================================");
        eprintln!("ANIMUS PACKER HATASI");
        eprintln!("========================================");
        eprintln!("{error}");
        eprintln!();

        process::exit(1);
    }
}


fn run() -> Result<(), Box<dyn Error>> {
    println!();
    println!("========================================");
    println!("        ANIMUS PACKAGE BUILDER v1");
    println!("========================================");
    println!();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    if args[1] == "--init-keys" {
        let signing_key = load_or_create_signing_key()?;

        let verifying_key = signing_key.verifying_key();

        println!("Animus Ed25519 anahtarı hazır.");
        println!(
            "Public Key: {}",
            BASE64.encode(verifying_key.to_bytes())
        );

        println!();

        return Ok(());
    }

    let input = PathBuf::from(&args[1]);

    if !input.is_file() {
        return Err(format!(
            "Kaynak dosya bulunamadı: {}",
            input.display()
        )
        .into());
    }

    let output = if args.len() >= 3 {
        PathBuf::from(&args[2])
    } else {
        default_output_path(&input)?
    };

    if output.exists() {
        return Err(format!(
            "Hedef dosya zaten mevcut:\n{}\n\n\
             Önce eski dosyayı sil veya farklı çıktı adı kullan.",
            output.display()
        )
        .into());
    }

    pack(&input, &output)?;

    Ok(())
}


// ============================================================
// USAGE
// ============================================================

fn print_usage() {
    println!("KULLANIM:");
    println!();

    println!("Anahtar oluştur:");
    println!(
        "  cargo run --release --bin animus-packer -- --init-keys"
    );

    println!();

    println!("ZIP paketle:");
    println!(
        "  cargo run --release --bin animus-packer -- \"Yama.zip\""
    );

    println!();

    println!("Özel çıktı:");
    println!(
        "  cargo run --release --bin animus-packer -- \
         \"Yama.zip\" \"Yama.animus\""
    );

    println!();
}


// ============================================================
// PACK
// ============================================================

fn pack(
    input: &Path,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    println!("Kaynak:");
    println!("  {}", input.display());

    println!();

    println!("Çıktı:");
    println!("  {}", output.display());

    println!();

    // --------------------------------------------------------
    // 1. Signing key
    // --------------------------------------------------------

    let signing_key =
        load_or_create_signing_key()?;

    let verifying_key =
        signing_key.verifying_key();

    let public_key_bytes =
        verifying_key.to_bytes();

    let public_key_b64 =
        BASE64.encode(public_key_bytes);

    let signing_key_id =
        signing_key_id(&public_key_bytes);

    println!(
        "Signing Key ID: {}",
        signing_key_id
    );

    println!();

    // --------------------------------------------------------
    // 2. Input metadata
    // --------------------------------------------------------

    let original_size =
        fs::metadata(input)?.len();

    let original_name =
        input
            .file_name()
            .ok_or("Dosya adı bulunamadı")?
            .to_string_lossy()
            .to_string();

    let file_name_bytes =
        original_name.as_bytes();

    if file_name_bytes.len() > u16::MAX as usize {
        return Err(
            "Dosya adı çok uzun.".into()
        );
    }

    println!(
        "ZIP boyutu: {} byte ({:.2} GB)",
        original_size,
        original_size as f64
            / 1024.0
            / 1024.0
            / 1024.0
    );

    // --------------------------------------------------------
    // 3. Original ZIP SHA-256
    // --------------------------------------------------------

    println!();
    println!(
        "[1/4] Orijinal ZIP SHA-256 hesaplanıyor..."
    );

    let original_sha256 =
        hash_file(input)?;

    println!(
        "ZIP SHA-256: {}",
        hex::encode(original_sha256)
    );

    // --------------------------------------------------------
    // 4. Package ID
    // --------------------------------------------------------

    let package_uuid =
        Uuid::new_v4();

    let package_id =
        *package_uuid.as_bytes();

    println!();

    println!(
        "Package ID: {}",
        package_uuid
    );

    // --------------------------------------------------------
    // 5. Encryption key
    // --------------------------------------------------------

    let mut encryption_key =
        [0u8; 32];

    OsRng.fill_bytes(
        &mut encryption_key
    );

    let cipher =
        XChaCha20Poly1305::new(
            Key::from_slice(&encryption_key)
        );

    // --------------------------------------------------------
    // 6. Nonce prefix
    // --------------------------------------------------------

    // 24-byte XChaCha nonce:
    //
    // 16 byte random prefix
    // +
    // 8 byte chunk counter

    let mut nonce_prefix =
        [0u8; 16];

    OsRng.fill_bytes(
        &mut nonce_prefix
    );

    // --------------------------------------------------------
    // 7. Temporary output
    // --------------------------------------------------------

    if let Some(parent) =
        output.parent()
    {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let temp_output =
        PathBuf::from(
            format!(
                "{}.tmp",
                output.display()
            )
        );

    if temp_output.exists() {
        fs::remove_file(
            &temp_output
        )?;
    }

    let mut output_file =
        File::create(
            &temp_output
        )?;

    // Bu hasher signature footer öncesindeki
    // paketin tamamını hash'ler.
    let mut payload_hasher =
        Sha256::new();

    // --------------------------------------------------------
    // 8. Header
    // --------------------------------------------------------

    println!();
    println!(
        "[2/4] Paket başlığı oluşturuluyor..."
    );

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        MAGIC,
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &FORMAT_VERSION.to_le_bytes(),
    )?;

    // flags
    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &0u16.to_le_bytes(),
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &(CHUNK_SIZE as u32)
            .to_le_bytes(),
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &original_size.to_le_bytes(),
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &package_id,
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &nonce_prefix,
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &original_sha256,
    )?;

    // Signing key ID:
    // SHA256(public_key)'in ilk 8 byte'ı.
    let signing_id_bytes =
        signing_key_id_bytes(
            &public_key_bytes
        );

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &signing_id_bytes,
    )?;

    let name_length =
        file_name_bytes.len() as u16;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        &name_length.to_le_bytes(),
    )?;

    write_hashed(
        &mut output_file,
        &mut payload_hasher,
        file_name_bytes,
    )?;

    // --------------------------------------------------------
    // 9. Encrypt chunks
    // --------------------------------------------------------

    println!();
    println!(
        "[3/4] ZIP şifreleniyor..."
    );

    let mut input_file =
        File::open(input)?;

    let mut buffer =
        vec![0u8; CHUNK_SIZE];

    let mut chunk_index =
        0u64;

    let mut processed =
        0u64;

    loop {
        let read =
            read_chunk(
                &mut input_file,
                &mut buffer,
            )?;

        if read == 0 {
            break;
        }

        let plaintext =
            &buffer[..read];

        let nonce =
            make_nonce(
                &nonce_prefix,
                chunk_index,
            );

        let aad =
            build_chunk_aad(
                &package_id,
                original_size,
                &original_sha256,
                chunk_index,
                read as u32,
            );

        let ciphertext =
            cipher
                .encrypt(
                    XNonce::from_slice(
                        &nonce
                    ),
                    Payload {
                        msg: plaintext,
                        aad: &aad,
                    },
                )
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        "XChaCha20-Poly1305 \
                         şifreleme başarısız."
                    )
                })?;

        let plain_length =
            read as u32;

        let cipher_length =
            ciphertext.len() as u32;

        // Chunk header
        write_hashed(
            &mut output_file,
            &mut payload_hasher,
            &plain_length.to_le_bytes(),
        )?;

        write_hashed(
            &mut output_file,
            &mut payload_hasher,
            &cipher_length.to_le_bytes(),
        )?;

        // Encrypted chunk
        write_hashed(
            &mut output_file,
            &mut payload_hasher,
            &ciphertext,
        )?;

        processed +=
            read as u64;

        chunk_index += 1;

        let percent =
            if original_size > 0 {
                processed as f64
                    * 100.0
                    / original_size as f64
            } else {
                100.0
            };

        print!(
            "\rŞifreleme: {:>6.2}%  \
             {:.2}/{:.2} GB",
            percent,
            processed as f64
                / 1024.0
                / 1024.0
                / 1024.0,
            original_size as f64
                / 1024.0
                / 1024.0
                / 1024.0
        );

        io::stdout().flush()?;
    }

    println!();

    if processed != original_size {
        encryption_key.zeroize();

        return Err(
            format!(
                "Kaynak boyutu değişti. \
                 Beklenen={} okunan={}",
                original_size,
                processed
            )
            .into()
        );
    }

    // --------------------------------------------------------
    // 10. Payload SHA
    // --------------------------------------------------------

    let payload_digest =
        payload_hasher
            .clone()
            .finalize();

    let mut payload_sha =
        [0u8; 32];

    payload_sha.copy_from_slice(
        &payload_digest
    );

    // --------------------------------------------------------
    // 11. Sign
    // --------------------------------------------------------

    println!();
    println!(
        "[4/4] Paket dijital olarak imzalanıyor..."
    );

    let sign_message =
        build_sign_message(
            &package_id,
            &payload_sha,
        );

    let signature =
        signing_key.sign(
            &sign_message
        );

    let signature_bytes =
        signature.to_bytes();

    // --------------------------------------------------------
    // 12. Footer
    // --------------------------------------------------------

    // package_hasher bütün final .animus
    // dosyasının SHA'sını hesaplar.
    let mut package_hasher =
        payload_hasher.clone();

    write_final(
        &mut output_file,
        &mut package_hasher,
        FOOTER_MAGIC,
    )?;

    write_final(
        &mut output_file,
        &mut package_hasher,
        &payload_sha,
    )?;

    write_final(
        &mut output_file,
        &mut package_hasher,
        &signature_bytes,
    )?;

    output_file.flush()?;
    output_file.sync_all()?;

    drop(output_file);

    // --------------------------------------------------------
    // 13. Final package SHA
    // --------------------------------------------------------

    let package_digest =
        package_hasher.finalize();

    let package_sha256 =
        hex::encode(
            package_digest
        );

    // --------------------------------------------------------
    // 14. Move temp -> final
    // --------------------------------------------------------

    fs::rename(
        &temp_output,
        output,
    )?;

    let package_size =
        fs::metadata(output)?.len();

    // --------------------------------------------------------
    // 15. Server-only key file
    // --------------------------------------------------------

    let encryption_key_b64 =
        BASE64.encode(
            encryption_key
        );

    let server_metadata =
        json!({
            "schema_version": 1,

            "package_id":
                package_uuid.to_string(),

            "format":
                "ANIMUS01",

            "encryption":
                "XChaCha20-Poly1305",

            "chunk_size":
                CHUNK_SIZE,

            "original_name":
                original_name,

            "original_size":
                original_size,

            "original_sha256":
                hex::encode(original_sha256),

            "package_size":
                package_size,

            "package_sha256":
                package_sha256,

            "encryption_key_b64":
                encryption_key_b64,

            "signing_public_key_b64":
                public_key_b64,

            "signing_key_id":
                signing_key_id,

            "created_at":
                Utc::now().to_rfc3339()
        });

    let server_key_path =
        PathBuf::from(
            format!(
                "{}.server-key.json",
                output.display()
            )
        );

    write_json_private(
        &server_key_path,
        &server_metadata,
    )?;

    // Anahtarı artık bellekte tutma.
    encryption_key.zeroize();

    // --------------------------------------------------------
    // 16. Finish
    // --------------------------------------------------------

    println!();

    println!(
        "========================================"
    );

    println!(
        "PAKET BAŞARIYLA OLUŞTURULDU"
    );

    println!(
        "========================================"
    );

    println!();

    println!(
        "ANIMUS DOSYASI:"
    );

    println!(
        "{}",
        output.display()
    );

    println!();

    println!(
        "Paket boyutu:"
    );

    println!(
        "{} byte ({:.2} GB)",
        package_size,
        package_size as f64
            / 1024.0
            / 1024.0
            / 1024.0
    );

    println!();

    println!(
        "Paket SHA-256:"
    );

    println!(
        "{}",
        package_sha256
    );

    println!();

    println!(
        "SERVER KEY DOSYASI:"
    );

    println!(
        "{}",
        server_key_path.display()
    );

    println!();

    println!(
        "UYARI:"
    );

    println!(
        ".server-key.json dosyasını \
         MEDIAFIRE veya GITHUB'a yükleme!"
    );

    println!();

    println!(
        "MediaFire'a sadece .animus \
         dosyasını yükle."
    );

    println!();

    Ok(())
}


// ============================================================
// INPUT HASH
// ============================================================

fn hash_file(
    path: &Path,
) -> Result<[u8; 32], Box<dyn Error>> {
    let mut file =
        File::open(path)?;

    let mut hasher =
        Sha256::new();

    let mut buffer =
        vec![0u8; 1024 * 1024];

    loop {
        let read =
            file.read(
                &mut buffer
            )?;

        if read == 0 {
            break;
        }

        hasher.update(
            &buffer[..read]
        );
    }

    let digest =
        hasher.finalize();

    let mut result =
        [0u8; 32];

    result.copy_from_slice(
        &digest
    );

    Ok(result)
}


// ============================================================
// CHUNK READER
// ============================================================

fn read_chunk(
    file: &mut File,
    buffer: &mut [u8],
) -> io::Result<usize> {
    let mut total =
        0usize;

    while total < buffer.len() {
        let read =
            file.read(
                &mut buffer[total..]
            )?;

        if read == 0 {
            break;
        }

        total += read;
    }

    Ok(total)
}


// ============================================================
// NONCE
// ============================================================

fn make_nonce(
    prefix: &[u8; 16],
    chunk_index: u64,
) -> [u8; 24] {
    let mut nonce =
        [0u8; 24];

    nonce[..16]
        .copy_from_slice(
            prefix
        );

    nonce[16..]
        .copy_from_slice(
            &chunk_index.to_le_bytes()
        );

    nonce
}


// ============================================================
// CHUNK AAD
// ============================================================

fn build_chunk_aad(
    package_id: &[u8; 16],
    original_size: u64,
    original_sha256: &[u8; 32],
    chunk_index: u64,
    plain_length: u32,
) -> Vec<u8> {
    let mut aad =
        Vec::with_capacity(
            8
                + 2
                + 16
                + 8
                + 32
                + 8
                + 4
        );

    aad.extend_from_slice(
        MAGIC
    );

    aad.extend_from_slice(
        &FORMAT_VERSION.to_le_bytes()
    );

    aad.extend_from_slice(
        package_id
    );

    aad.extend_from_slice(
        &original_size.to_le_bytes()
    );

    aad.extend_from_slice(
        original_sha256
    );

    aad.extend_from_slice(
        &chunk_index.to_le_bytes()
    );

    aad.extend_from_slice(
        &plain_length.to_le_bytes()
    );

    aad
}


// ============================================================
// SIGN MESSAGE
// ============================================================

fn build_sign_message(
    package_id: &[u8; 16],
    payload_sha256: &[u8; 32],
) -> Vec<u8> {
    let mut message =
        Vec::with_capacity(
            SIGN_DOMAIN.len()
                + 16
                + 32
        );

    message.extend_from_slice(
        SIGN_DOMAIN
    );

    message.extend_from_slice(
        package_id
    );

    message.extend_from_slice(
        payload_sha256
    );

    message
}


// ============================================================
// HASHED WRITES
// ============================================================

fn write_hashed(
    file: &mut File,
    hasher: &mut Sha256,
    bytes: &[u8],
) -> io::Result<()> {
    file.write_all(bytes)?;

    hasher.update(bytes);

    Ok(())
}


fn write_final(
    file: &mut File,
    hasher: &mut Sha256,
    bytes: &[u8],
) -> io::Result<()> {
    file.write_all(bytes)?;

    hasher.update(bytes);

    Ok(())
}


// ============================================================
// DEFAULT OUTPUT
// ============================================================

fn default_output_path(
    input: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let parent =
        input.parent()
            .unwrap_or_else(
                || Path::new(".")
            );

    let stem =
        input
            .file_stem()
            .ok_or(
                "Kaynak dosyanın adı bulunamadı"
            )?
            .to_string_lossy();

    Ok(
        parent.join(
            format!(
                "{stem}.animus"
            )
        )
    )
}


// ============================================================
// SIGNING KEY STORAGE
// ============================================================

fn signing_key_directory(
) -> Result<PathBuf, Box<dyn Error>> {
    let root =
        dirs::data_local_dir()
            .ok_or(
                "Windows LocalAppData bulunamadı."
            )?
            .join(
                "AnimusPatchLoader"
            )
            .join(
                "packer-keys"
            );

    fs::create_dir_all(
        &root
    )?;

    Ok(root)
}


fn load_or_create_signing_key(
) -> Result<SigningKey, Box<dyn Error>> {
    let dir =
        signing_key_directory()?;

    let private_path =
        dir.join(
            "ed25519-signing.key"
        );

    let public_path =
        dir.join(
            "ed25519-signing.pub"
        );

    // --------------------------------------------------------
    // Existing key
    // --------------------------------------------------------

    if private_path.is_file() {
        let bytes =
            fs::read(
                &private_path
            )?;

        if bytes.len() != 32 {
            return Err(
                format!(
                    "Signing private key bozuk: {}",
                    private_path.display()
                )
                .into()
            );
        }

        let mut seed =
            [0u8; 32];

        seed.copy_from_slice(
            &bytes
        );

        let signing =
            SigningKey::from_bytes(
                &seed
            );

        seed.zeroize();

        return Ok(signing);
    }

    // --------------------------------------------------------
    // Create key
    // --------------------------------------------------------

    println!(
        "İlk kullanım: Animus Ed25519 \
         signing key oluşturuluyor..."
    );

    let mut seed =
        [0u8; 32];

    OsRng.fill_bytes(
        &mut seed
    );

    let signing =
        SigningKey::from_bytes(
            &seed
        );

    let verifying =
        signing.verifying_key();

    // create_new:
    // yanlışlıkla mevcut anahtarı ezmez.
    {
        let mut file =
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(
                    &private_path
                )?;

        file.write_all(
            &seed
        )?;

        file.flush()?;
        file.sync_all()?;
    }

    fs::write(
        &public_path,
        verifying.to_bytes(),
    )?;

    seed.zeroize();

    println!();

    println!(
        "Private key:"
    );

    println!(
        "{}",
        private_path.display()
    );

    println!();

    println!(
        "Public key:"
    );

    println!(
        "{}",
        public_path.display()
    );

    println!();

    println!(
        "PRIVATE KEY DOSYASINI \
         YEDEKLE VE PAYLAŞMA."
    );

    println!();

    Ok(signing)
}


// ============================================================
// SIGNING KEY ID
// ============================================================

fn signing_key_id_bytes(
    public_key: &[u8; 32],
) -> [u8; 8] {
    let digest =
        Sha256::digest(
            public_key
        );

    let mut id =
        [0u8; 8];

    id.copy_from_slice(
        &digest[..8]
    );

    id
}


fn signing_key_id(
    public_key: &[u8; 32],
) -> String {
    hex::encode(
        signing_key_id_bytes(
            public_key
        )
    )
}


// ============================================================
// PRIVATE JSON
// ============================================================

fn write_json_private(
    path: &Path,
    value: &serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let json =
        serde_json::to_vec_pretty(
            value
        )?;

    let mut file =
        File::create(path)?;

    file.write_all(
        &json
    )?;

    file.flush()?;
    file.sync_all()?;

    Ok(())
}
