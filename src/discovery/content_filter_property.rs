use serde::{Serialize, Deserialize};
use crate::structure::parameter_id::ParameterId;

/// The ContentFilterProperty field provides all the required information to
/// enable content filtering on the Writer side. For example, for the default
/// DDSSQL filter class, a valid filter expression for a data type containing
/// members a, b and c could be “(a < 5) AND (b == %0) AND (c >= %1)” with
/// expression parameters “5” and “3.” In order for the Writer to apply
/// the filter, it must have been configured to handle filters of the specified
/// filter class. If not, the Writer will simply ignore the filter information
/// and not filter any data samples.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentFilterProperty {
  /// Name of the Content-filtered Topic associated with the Reader.
  /// Must have non-zero length.
  pub contentFilteredTopicName: String,

  /// Name of the Topic related to the Content-filtered Topic.
  /// Must have non-zero length.
  pub relatedTopicName: String,

  /// Identifies the filter class this filter belongs to. RTPS can support
  /// multiple filter classes (SQL, regular expressions, custom filters,
  /// etc). Must have non-zero length.
  /// RTPS predefines the following values:
  /// “DDSSQL” Default filter class name if none specified.
  /// Matches the SQL filter specified by DDS, which must be available in all
  /// implementations.
  pub filterClassName: String,

  /// The actual filter expression. Must be a valid expression for the filter
  /// class specified using filterClassName.
  /// Must have non-zero length.
  pub filterExpression: String,

  /// Defines the value for each parameter in the filter expression.
  /// Can have zero length if the filter expression contains no parameters.
  pub expressionParameters: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ContentFilterPropertyData {
  parameter_id: ParameterId,
  parameter_length: u16,
  content_filter_property: ContentFilterProperty,
}

impl ContentFilterPropertyData {
  pub fn new(content_filter_property: &ContentFilterProperty) -> ContentFilterPropertyData {
    let len_cftn = content_filter_property.contentFilteredTopicName.len();
    let len_cftn = len_cftn + (4 - len_cftn % 4) + 4;
    let len_rtn = content_filter_property.relatedTopicName.len();
    let len_rtn = len_rtn + (4 - len_rtn % 4) + 4;
    let len_fcn = content_filter_property.filterClassName.len();
    let len_fcn = len_fcn + (4 - len_fcn % 4) + 4;
    let len_fe = content_filter_property.filterExpression.len();
    let len_fe = len_fe + (4 - len_fe % 4) + 4;

    let mut parameter_length = len_cftn + len_rtn + len_fcn + len_fe;
    for param in content_filter_property.expressionParameters.iter() {
      let len_temp = param.len();
      let len_temp = len_temp + (4 - len_temp % 4) + 4;
      parameter_length = parameter_length + len_temp;
    }

    parameter_length = parameter_length + (4 - parameter_length % 4) + 4;

    ContentFilterPropertyData {
      parameter_id: ParameterId::PID_CONTENT_FILTER_PROPERTY,
      parameter_length: parameter_length as u16,
      content_filter_property: content_filter_property.clone(),
    }
  }
}
