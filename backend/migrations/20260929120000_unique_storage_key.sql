-- Each stored object backs exactly one file version: two versions sharing a
-- key would let purging one of them delete the other's content.
CREATE UNIQUE INDEX unique_storage_key ON file_versions (storage_key);
