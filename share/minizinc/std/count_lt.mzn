include "fzn_count_lt.mzn";
include "fzn_count_lt_par.mzn";
include "fzn_count_lt_reif.mzn";
include "fzn_count_lt_par_reif.mzn";

/** @group globals.counting
  Constrains \a c to be strictly less than the number of occurrences
  of \a y in \a x. 
*/
predicate count_lt(array[$X] of var $$E: x, var $$E: y, var int: c) =
    fzn_count_lt(erase_enum(array1d(x)),erase_enum(y),c);

/** @group globals.counting
  Constrains \a c to be strictly less than the number of occurrences
  of \a y in \a x. 
*/
predicate count_lt(array[$X] of var $$E: x, $$E: y, int: c) =
    fzn_count_lt_par(erase_enum(array1d(x)),erase_enum(y),c);

%-----------------------------------------------------------------------------%
