use crate::{Asset, AssetId, AssetType, LoadSettings};

pub trait AsBytes {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl<A: serde::Serialize + serde::de::DeserializeOwned> AsBytes for A {
    fn to_bytes(&self) -> Vec<u8> {
        let string = toml::to_string(self).unwrap();
        string.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap()
    }
}

pub struct Header {
    pub version: u32,
    pub settings_size: u32,
    pub data_size: u32,
    pub data_type: AssetType,
    pub id: AssetId,
}

pub struct AssetPack {
    header: Header,
    blob: Vec<u8>,
}

impl AssetPack {
    pub fn new<A: Asset>(id: AssetId, settings: Vec<u8>, data: Vec<u8>) -> Self {
        Self {
            header: Header {
                version: 1,
                settings_size: settings.len() as u32,
                data_size: data.len() as u32,
                data_type: AssetType::new::<A>(),
                id,
            },
            blob: [settings, data].concat(),
        }
    }

    pub fn from_asset<A: Asset + AsBytes, S: LoadSettings>(
        id: AssetId,
        asset: &A,
        settings: &S,
    ) -> Self {
        let asset_bytes = asset.to_bytes().to_vec();
        let settings_bytes = settings.to_bytes().to_vec();

        Self::new::<A>(id, settings_bytes, asset_bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let header = Header {
            version: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            settings_size: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            data_size: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            data_type: u64::from_le_bytes(bytes[12..20].try_into().unwrap()).into(),
            id: u64::from_le_bytes(bytes[20..28].try_into().unwrap()).into(),
        };

        let header_size = std::mem::size_of::<Header>();

        let settings = bytes[header_size..header_size + header.settings_size as usize].to_vec();
        let data = bytes[header_size + header.settings_size as usize..].to_vec();

        Self {
            header,
            blob: [settings, data].concat(),
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn settings(&self) -> &[u8] {
        &self.blob[0..self.header.settings_size as usize]
    }

    pub fn data(&self) -> &[u8] {
        &self.blob[self.header.settings_size as usize..]
    }

    pub fn metadata(&self) -> &[u8] {
        &self.blob[0..self.header.settings_size as usize + std::mem::size_of::<AssetId>()]
    }
}
