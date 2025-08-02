use std::path::PathBuf;

use tokio::task::JoinHandle;

use crate::{
    allow_list::AllowList,
    stages::{
        event::Dispatcher, filter_stage::FilterStage, map_stage::MapStage, router::StageRegistry, save_file::SaveFileStage, scan_stage::ScanStage, stage::StageId
    },
};

pub fn initialize_stages(allow_list: AllowList, output: PathBuf) -> (Dispatcher, JoinHandle<()>) {
    StageRegistry::default()
        .register(
            StageId::Filter,
            Box::new(FilterStage::new(allow_list)),
        )
        .register(StageId::Map, Box::new(MapStage::new()))
        .register(StageId::SaveFile, Box::new(SaveFileStage::new(output)))
        .register(StageId::Scan, Box::new(ScanStage::new()))
        .build()
}
