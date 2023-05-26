use speedy::{Context, Readable, Reader, Writable, Writer};

use crate::serialization::speedy_pl_cdr_helpers::*;

/// The ContentFilterProperty field provides all the required information to
/// enable content filtering on the Writer side. For example, for the default
/// DDSSQL filter class, a valid filter expression for a data type containing
/// members a, b and c could be “(a < 5) AND (b == %0) AND (c >= %1)” with
/// expression parameters “5” and “3.” In order for the Writer to apply
/// the filter, it must have been configured to handle filters of the specified
/// filter class. If not, the Writer will simply ignore the filter information
/// and not filter any data samples.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentFilterProperty {
  /// Name of the Content-filtered Topic associated with the Reader.
  /// Must have non-zero length.
  pub content_filtered_topic_name: String,

  /// Name of the Topic related to the Content-filtered Topic.
  /// Must have non-zero length.
  pub related_topic_name: String,

  /// Identifies the filter class this filter belongs to. RTPS can support
  /// multiple filter classes (SQL, regular expressions, custom filters,
  /// etc). Must have non-zero length.
  /// RTPS predefines the following values:
  /// “DDSSQL” Default filter class name if none specified.
  /// Matches the SQL filter specified by DDS, which must be available in all
  /// implementations.
  pub filter_class_name: String,

  /// The actual filter expression. Must be a valid expression for the filter
  /// class specified using filter_class_name.
  /// Must have non-zero length.
  pub filter_expression: String,

  /// Defines the value for each parameter in the filter expression.
  /// Can have zero length if the filter expression contains no parameters.
  pub expression_parameters: Vec<String>,
}

// These are for PL_CD (de)serialization

impl<'a, C: Context> Readable<'a, C> for ContentFilterProperty {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {

    let cftn: StringWithNul = reader.read_value()?;
    let rtn: StringWithNul = reader.read_value()?;
    let fcn: StringWithNul = reader.read_value()?;
    let fe: StringWithNul = reader.read_value()?;
    let eps: Vec<StringWithNul> = reader.read_value()?;

    Ok(ContentFilterProperty{
      content_filtered_topic_name: cftn.into(),
      related_topic_name: rtn.into(),
      filter_class_name: fcn.into(),
      filter_expression: fe.into(),
      expression_parameters: eps.into_iter().map( String::from ).collect(),
    })
  }

}

impl<C: Context> Writable<C> for ContentFilterProperty {
  fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
    writer.write_value(&StringWithNul::from(self.content_filtered_topic_name.clone()))?;
    writer.write_value(&StringWithNul::from(self.related_topic_name.clone()))?;
    writer.write_value(&StringWithNul::from(self.filter_class_name.clone()))?;
    writer.write_value(&StringWithNul::from(self.filter_expression.clone()))?;

    writer.write_value(
      &self.expression_parameters.iter().cloned()
      .map(StringWithNul::from).collect::<Vec<StringWithNul>>()
    )?;
    Ok(())
  }
}
