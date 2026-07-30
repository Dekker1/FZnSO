// A downstream codebase with a runtime-tagged value type feeds it across the
// interface through `fznso::ValueSource`, without copying its payload.
//
// This is the pattern an interpreter (such as MiniZinc, whose `Expression` is a
// tagged union) uses: one type is the value source, a custom `Model` returns it
// from its accessors, and it round-trips through the interface's vtables read
// back via `ModelRefAdapter`.

#include "fznso.hpp"

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <string_view>
#include <filesystem>
#include <vector>

namespace {

void check(bool condition, const char* what) {
	if (!condition) {
		std::fprintf(stderr, "FAILED: %s\n", what);
		std::abort();
	}
}

/// Set an environment variable. `setenv` is POSIX; Windows spells it `_putenv_s`.
void set_env(const char* name, const char* value) {
#if defined(_WIN32)
	(void)::_putenv_s(name, value);
#else
	(void)::setenv(name, value, 1);
#endif
}

/// Remove an environment variable; on Windows an empty value unsets it.
void unset_env(const char* name) {
#if defined(_WIN32)
	(void)::_putenv_s(name, "");
#else
	(void)::unsetenv(name);
#endif
}

/// A tagged-union expression that presents itself as a value, borrowing its own
/// storage rather than copying into a `OwnedValue`.
struct Expression final : fznso::ValueSource {
	enum class Tag { Int, Bool, Float, Str, Array };
	Tag tag = Tag::Int;
	std::int64_t i = 0;
	bool b = false;
	double f = 0;
	std::string s;
	std::vector<Expression> items;

	FznsoValueKind kind() const override {
		switch (tag) {
		case Tag::Int:
			return FznsoValueInt;
		case Tag::Bool:
			return FznsoValueBool;
		case Tag::Float:
			return FznsoValueFloat;
		case Tag::Str:
			return FznsoValueString;
		case Tag::Array:
			return FznsoValueList;
		}
		return FznsoValueAbsent;
	}
	std::int64_t as_int() const override { return i; }
	bool as_bool() const override { return b; }
	double as_float() const override { return f; }
	std::string_view as_string() const override { return s; }
	std::size_t size() const override { return items.size(); }
	fznso::Value element(std::size_t index) const override { return fznso::Value{items[index]}; }

	static Expression integer(std::int64_t v) {
		Expression e;
		e.tag = Tag::Int;
		e.i = v;
		return e;
	}
	static Expression text(std::string v) {
		Expression e;
		e.tag = Tag::Str;
		e.s = std::move(v);
		return e;
	}
	static Expression array(std::vector<Expression> v) {
		Expression e;
		e.tag = Tag::Array;
		e.items = std::move(v);
		return e;
	}
};

/// A minimal model over expression-valued constraint arguments, implementing
/// the [`fznso::Model`] interface directly rather than copying into a
/// [`fznso::LayeredModel`].
class ExprModel final : public fznso::Model {
public:
	void add(std::string ident, std::vector<Expression> arguments) {
		constraints_.push_back({std::move(ident), std::move(arguments)});
	}

	std::size_t layer_count() const override { return 1; }
	std::size_t layer_permanent() const override { return 1; }
	std::size_t layer_unchanged() const override { return 0; }
	std::size_t layer_redundant_count() const override { return 0; }
	std::size_t layer_redundant_index(std::size_t) const override { return 0; }

	std::size_t decision_count() const override { return 0; }
	std::size_t decision_layer_end(std::size_t) const override { return 0; }
	fznso::Value decision_domain(fznso::Decision) const override { return fznso::Value{}; }
	std::optional<std::string_view> decision_name(fznso::Decision) const override {
		return std::nullopt;
	}
	bool decision_defined(fznso::Decision) const override { return false; }
	std::size_t decision_annotation_count(fznso::Decision) const override { return 0; }
	fznso::AnnotationRef decision_annotation(fznso::Decision, std::size_t) const override {
		std::abort();
	}

	std::size_t constraint_count() const override { return constraints_.size(); }
	std::size_t constraint_layer_end(std::size_t) const override { return constraints_.size(); }
	std::string_view constraint_ident(fznso::Constraint c) const override {
		return constraints_[c.index].ident;
	}
	std::size_t constraint_argument_count(fznso::Constraint c) const override {
		return constraints_[c.index].arguments.size();
	}
	fznso::Value constraint_argument(fznso::Constraint c, std::size_t i) const override {
		return fznso::Value{constraints_[c.index].arguments[i]};
	}
	std::optional<fznso::Decision> constraint_defines(fznso::Constraint) const override {
		return std::nullopt;
	}
	std::size_t constraint_annotation_count(fznso::Constraint) const override { return 0; }
	fznso::AnnotationRef constraint_annotation(fznso::Constraint, std::size_t) const override {
		std::abort();
	}

	std::string_view objective_ident() const override { return {}; }
	fznso::Value objective_arg() const override { return fznso::Value{}; }
	std::size_t objective_annotation_count() const override { return 0; }
	fznso::AnnotationRef objective_annotation(std::size_t) const override { std::abort(); }

private:
	struct ConstraintData {
		std::string ident;
		std::vector<Expression> arguments;
	};
	std::vector<ConstraintData> constraints_;
};

} // namespace

int main() {
	// Direct conversion of each kind, borrowing the expression's own storage.
	Expression ei = Expression::integer(-9000000000LL);
	check(fznso::Value{ei}.as_int() == -9000000000LL, "int expression");

	Expression es = Expression::text("minizinc");
	fznso::Value sv = fznso::Value{es};
	check(sv.kind() == FznsoValueString && sv.as_string() == "minizinc", "string expression");

	Expression arr = Expression::array(
		{Expression::integer(10), Expression::integer(20), Expression::integer(30)});
	fznso::Value lv = fznso::Value{arr};
	check(lv.kind() == FznsoValueList && lv.size() == 3, "array expression");
	check(lv[0].as_int() == 10 && lv[2].as_int() == 30, "array elements");

	// The values survive a round trip through the model vtable: a custom model
	// hands them out, and they are read back as if received across the interface.
	ExprModel model;
	model.add("int_le", {Expression::integer(3), Expression::text("x")});
	model.add("array_eq", {Expression::array({Expression::integer(1), Expression::integer(2)})});

	fznso::ModelRefAdapter back{fznso::model_ref(model)};
	check(back.constraint_count() == 2, "adapter sees both constraints");
	check(back.constraint_ident(fznso::Constraint{0}) == "int_le", "constraint ident");
	fznso::Value arg0 = back.constraint_argument(fznso::Constraint{0}, 0);
	fznso::Value arg1 = back.constraint_argument(fznso::Constraint{0}, 1);
	check(arg0.as_int() == 3, "int argument round trips");
	check(arg1.as_string() == "x", "string argument round trips");
	fznso::Value arg2 = back.constraint_argument(fznso::Constraint{1}, 0);
	check(arg2.kind() == FznsoValueList && arg2.size() == 2, "array argument round trips");
	check(arg2[1].as_int() == 2, "nested array element round trips");

	// An OwnedValue copies a borrowed value into storage that outlives its
	// source, recursively, so the copy is safe after the source is gone.
	fznso::OwnedValue snapshot = [] {
		Expression tree = Expression::array(
			{Expression::integer(1), Expression::text("two"),
		     Expression::array({Expression::integer(3), Expression::integer(4)})});
		return fznso::OwnedValue{fznso::Value{tree}};
	}();  // the source `tree` is destroyed here

	fznso::Value copied{snapshot};
	check(copied.kind() == FznsoValueList && copied.size() == 3, "owned copy is a list");
	check(copied[0].as_int() == 1, "owned copy scalar");
	check(copied[1].as_string() == "two", "owned copy string");
	check(copied[2].kind() == FznsoValueList && copied[2][1].as_int() == 4, "owned copy nested");

	// A downstream annotation type presents itself through AnnotationSource, the
	// same way an expression does through ValueSource.
	struct OutputVar final : fznso::AnnotationSource {
		Expression arg = Expression::integer(5);
		std::string_view ident() const override { return "output_var"; }
		std::size_t size() const override { return 1; }
		fznso::Value argument(std::size_t) const override { return fznso::Value{arg}; }
	};
	OutputVar annotation;
	fznso::AnnotationRef ann{annotation};
	check(ann.ident() == "output_var", "annotation source ident");
	check(ann.size() == 1 && ann[0].as_int() == 5, "annotation source argument");

	// Every value hands out a *complete* vtable: the interface declares the slots
	// as plain function pointers, so a null one is not a valid entry — a consumer
	// in another language (Rust's `extern "C" fn` fields are non-nullable) may
	// reject the table outright rather than read the null as "unsupported".
	// Unimplemented slots point at an aborting stub instead.
	auto check_complete = [](fznso::Value v, const char* what) {
		const FznsoValueMethods* m = v.raw().methods;
		check(m != nullptr, what);
		check(m->kind != nullptr && m->len != nullptr, what);
		check(m->as_decision != nullptr && m->as_constraint != nullptr, what);
		check(m->as_int != nullptr && m->as_float != nullptr, what);
		check(m->as_string != nullptr && m->as_bool != nullptr, what);
		check(m->get_range_int != nullptr && m->get_range_float != nullptr, what);
		check(m->get_element != nullptr, what);
	};
	const bool a_bool = true;
	const std::int64_t an_int = 1;
	const double a_float = 1.5;
	const std::string a_string = "s";
	const std::vector<fznso::Range<std::int64_t>> int_ranges{{1, 2}};
	const std::vector<fznso::Range<double>> float_ranges{{1.0, 2.0}};
	const std::vector<std::int64_t> a_list{1, 2};
	check_complete(fznso::Value{}, "absent vtable complete");
	check_complete(fznso::Value{a_bool}, "bool vtable complete");
	check_complete(fznso::Value{an_int}, "int vtable complete");
	check_complete(fznso::Value{a_float}, "float vtable complete");
	check_complete(fznso::Value{a_string}, "string vtable complete");
	check_complete(fznso::Value{fznso::Decision{3}}, "decision vtable complete");
	check_complete(fznso::Value{fznso::Constraint{4}}, "constraint vtable complete");
	check_complete(fznso::Value{int_ranges}, "int-set vtable complete");
	check_complete(fznso::Value{float_ranges}, "float-set vtable complete");
	check_complete(fznso::Value{a_list}, "list vtable complete");
	check_complete(fznso::Value{ei}, "value-source vtable complete");

	// A solver's name must spell the `fznso_<name>_` entry points, so the file
	// name is validated up front rather than failing later as a missing symbol.
	// The platform's versioning schemes all name `gecode`, which is what lets
	// several versions sit side by side.
	for (const char* path : {"libgecode.so", "libgecode.so.6", "libgecode.6.2.1.dylib",
	                         "gecode.dll", "/usr/lib/fznso/libgecode.so.6"}) {
		check(fznso::detail::name_from_path(path) == "gecode", "derives gecode");
		check(fznso::detail::is_valid_name(fznso::detail::name_from_path(path)), "gecode is usable");
	}
	check(!fznso::detail::is_valid_name(fznso::detail::name_from_path(".hidden.so")),
	      "a dotfile yields no usable name");
	check(!fznso::detail::is_valid_name(fznso::detail::name_from_path("gecode-6.dll")),
	      "a versioned Windows name is rejected");
	check(!fznso::detail::is_valid_name("2fast"), "a leading digit is rejected");
	check(fznso::detail::is_valid_name("or_tools2"), "digits and underscores are fine");

	// The version is parsed out of the file name, and requests match by component.
	{
		std::string marker{fznso::detail::lib_marker()};
		check(fznso::detail::version_from_path(("libgecode" + marker + ".6.2.1").c_str()) == "6.2.1",
		      "version parsed");
		check(fznso::detail::version_from_path(("libgecode" + marker).c_str()).empty(),
		      "unversioned yields empty");
	}
	check(fznso::detail::version_matches("", "6.2.1"), "empty matches anything");
	check(fznso::detail::version_matches("6", "6.2.1"), "6 selects 6.2.1");
	check(fznso::detail::version_matches("6.2", "6.2.1"), "6.2 selects 6.2.1");
	check(!fznso::detail::version_matches("6", "60"), "6 does not select 60");
	check(!fznso::detail::version_matches("6.2", "6.1.0"), "6.2 does not select 6.1.0");
	check(!fznso::detail::version_matches("6.2.1.0", "6.2.1"), "more specific request fails");

	// Versions order naturally: 10 after 9, and zero-padding is insignificant.
	check(fznso::detail::natural_less("libx.so.9", "libx.so.10"), "9 sorts before 10");
	check(!fznso::detail::natural_less("libx.so.10", "libx.so.9"), "10 sorts after 9");
	check(!fznso::detail::natural_less("libx.so.8", "libx.so.08") &&
	          !fznso::detail::natural_less("libx.so.08", "libx.so.8"),
	      "leading zeros do not matter");

	// A directory scan skips non-libraries and unusable names, newest version
	// first per solver. `discover` never opens anything, so fake files suffice.
	{
		namespace fs = std::filesystem;
		fs::path dir = fs::temp_directory_path() / "fznso-cpp-discover";
		fs::remove_all(dir);
		fs::create_directories(dir);
		std::string marker{fznso::detail::lib_marker()};
		// `gecode` uses the convention (version before the extension); `chuffed`
		// carries none, and the native form is included to show it still parses.
		for (const std::string& f : {"libgecode.6" + marker, "libgecode.10" + marker,
		                             "libchuffed" + marker, std::string{"README.txt"},
		                             "lib2fast" + marker}) {
			std::ofstream{dir / f};
		}
		std::string saved = fznso::detail::env("FZNSO_SOLVER_PATH");
		set_env("FZNSO_SOLVER_PATH", dir.string().c_str());
		std::vector<fznso::Library::Discovered> found = fznso::Library::discover();
		check(found.size() == 3, "three usable libraries discovered");
		check(found[0].name == "chuffed" && found[0].version.empty(), "chuffed sorts first");
		check(found[1].name == "gecode" && found[1].version == "10", "gecode 10 comes first");
		check(found[2].name == "gecode" && found[2].version == "6", "then gecode 6");
		if (saved.empty()) {
			unset_env("FZNSO_SOLVER_PATH");
		} else {
			set_env("FZNSO_SOLVER_PATH", saved.c_str());
		}
		fs::remove_all(dir);
	}

	std::printf("all value-source checks passed\n");
	return 0;
}
