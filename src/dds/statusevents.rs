//
// Describe the commnucation status changes as events.
//
// These implement a mechanism equivalnet to what is described in
// Section 2.2.4 Listeners, Conditions, and Wait-sets
//
// Communcation statues are detailed in Figure 2.13 and tables in Section 2.2.4.1
// in DDS Specification v1.4

use crate::dds::qos::QosPolicyId;

pub enum TopicStatus {
	InconsistentTopic { count: CountAndChange },
}


pub enum DataReaderStatus {
	SampleRejected { 
		count: CountAndChange,
		last_reason: SampleRejectedStatusKind,
		//last_instance_key:  
	},
	LivelinessChanged { 
		alive_total: CountAndChange,
		not_alive_total: CountAndChange,
		//last_publication_key:  
	},
	RequestedDeadlineMissed { 
		count: CountAndChange,
		//last_instance_key:  
	},
	RequestedIncompatibleQos { 
		count: CountAndChange,
		last_policy_id: QosPolicyId,
		policies: Vec<QosPolicyCount>,
	},
	// DataAvailable is not implemented, as it seems to bring little additional value, 
	// because the normal data waiting mechanism already uses the same mio::poll structure,
	// so repeating the functionality here would bring little additional value.
	SampleLost  { 
		count: CountAndChange 
	},
	SubscriptionMatched { 
		total: CountAndChange,
		current: CountAndChange,
		//last_publication_key:  
	},
}


pub enum DataWriterStatus {
	LivelinessLost { 
		count: CountAndChange 
	},
	OfferedDeadlineMissed { 
		count: CountAndChange,
		//last_instance_key:  
	},
	OfferedIncompatibleQos { 
		count: CountAndChange,
		last_policy_id: QosPolicyId,
		policies: Vec<QosPolicyCount>,  
	},
	PublicationMatched { 
		total: CountAndChange,
		current: CountAndChange,
		//last_subscription_key:  
	},
}


// helper for counts
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountAndChange {
	// 2.3. Platform Specific Model defines these as "long", which appears to be 32-bit signed.
	pub count: i32, 
	pub change: i32, 
}

// sample rejection reasons
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleRejectedStatusKind {
	NotRejected,
	ByInstancesLimit,
	BySamplesLimit,
	BySamplesPerInstanceLimit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QosPolicyCount {
	policy_id: QosPolicyId,
	count: i32,	
}
