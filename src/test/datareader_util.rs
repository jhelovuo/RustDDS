// use serde::de::DeserializeOwned;
// use mio_extras::channel as mio_channel;

// use crate::{
//   dds::{With_Key_DataReader as DataReader},
//   dds::statusevents::DataReaderStatus,
//   dds::traits::Keyed,
//   serialization::DeserializerAdapter,
//   dds::with_key::ReaderCommand,
// };


//   This file was helper methods for setting the receiver and commander fields
//   on a DataReader. These fields are now private so this can't exist and the tests
//   using it need to be rewritten


// #[cfg(test)]
// pub(crate) trait DataReaderTestUtil {
//   fn set_status_change_receiver(&mut self, receiver: mio_channel::Receiver<DataReaderStatus>);
//   fn set_reader_commander(&mut self, commander: mio_channel::SyncSender<ReaderCommand>);
// }



// #[cfg(test)]
// impl<D, DA> DataReaderTestUtil for DataReader<D, DA>
// where
//   D: Keyed + DeserializeOwned,
//   DA: DeserializerAdapter<D>,
// {
//   fn set_status_change_receiver(&mut self, receiver: mio_channel::Receiver<DataReaderStatus>) {
//     self.status_receiver = receiver;
//   }

//   fn set_reader_commander(&mut self, commander: mio_channel::SyncSender<ReaderCommand>) {
//     self.reader_command = commander;
//   }
// }
