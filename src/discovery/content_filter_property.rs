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

fn read_pad<'a, C: Context, R: Reader<'a, C>>(reader: &mut R, read_length: usize, align:usize)
  -> Result<(), C::Error>
{
  let m = read_length % align;
  if m > 0 {
    reader.skip_bytes(align - m)?;
  }
  Ok(())
}

// TODO: This is patchy hack. 
// Speedy reader/writer implementations do not respect
// alignment. A string starts with 4-byte character count, so
// we align to 4 bytes BEFORE each string reading operation, by the
// misalignment caused previously.
// Note: we must align BEFORE string read, not after.
// If string were followed by e.g. "u8", that would not have alignment applied.
impl<'a, C: Context> Readable<'a, C> for ContentFilterProperty {
  fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {

    let cftn: StringWithNul = reader.read_value()?;
    read_pad(reader, cftn.len(), 4)?;

    let rtn: StringWithNul = reader.read_value()?;
    read_pad(reader, rtn.len(), 4)?;

    let fcn: StringWithNul = reader.read_value()?;
    read_pad(reader, fcn.len(), 4)?;

    let fe: StringWithNul = reader.read_value()?;
    read_pad(reader, fe.len(), 4)?;

    let mut eps : Vec<String> = Vec::with_capacity(2);
    let count = reader.read_u32()?;
    for _ in 0..count {
      let s : StringWithNul = reader.read_value()?;
      read_pad(reader, s.len(), 4)?;
      eps.push( s.into() );
    }
    //let eps: Vec<StringWithNul> = reader.read_value()?;

    Ok(ContentFilterProperty{
      content_filtered_topic_name: cftn.into(),
      related_topic_name: rtn.into(),
      filter_class_name: fcn.into(),
      filter_expression: fe.into(),
      expression_parameters: eps,
    })
  }

}

// TODO: Write alignment missing.
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
