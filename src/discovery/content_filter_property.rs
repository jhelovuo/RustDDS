/// The ContentFilterProperty_t field provides all the required information to
/// enable content filtering on the Writer side. For example, for the default
/// DDSSQL filter class, a valid filter expression for a data type containing
/// members a, b and c could be “(a < 5) AND (b == %0) AND (c >= %1)” with
/// expression parameters “5” and “3.” In order for the Writer to apply
/// the filter, it must have been configured to handle filters of the specified
/// filter class. If not, the Writer will simply ignore the filter information
/// and not filter any data samples.
pub struct ContentFilterProperty_t {
  /// Name of the Content-filtered Topic associated with the Reader.
  /// Must have non-zero length.
  contentFilteredTopicName: String,

  /// Name of the Topic related to the Content-filtered Topic.
  /// Must have non-zero length.
  relatedTopicName: String,

  /// Identifies the filter class this filter belongs to. RTPS can support
  /// multiple filter classes (SQL, regular expressions, custom filters,
  /// etc). Must have non-zero length.
  /// RTPS predefines the following values:
  /// “DDSSQL” Default filter class name if none specified.
  /// Matches the SQL filter specified by DDS, which must be available in all
  /// implementations.
  filterClassName: String,

  /// The actual filter expression. Must be a valid expression for the filter
  /// class specified using filterClassName.
  /// Must have non-zero length.
  filterExpression: String,

  /// Defines the value for each parameter in the filter expression.
  /// Can have zero length if the filter expression contains no parameters.
  expressionParameters: Vec<String>,
}
