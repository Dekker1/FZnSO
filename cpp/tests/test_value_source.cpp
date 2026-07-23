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
#include <string>
#include <string_view>
#include <vector>

namespace {

void check(bool condition, const char* what) {
	if (!condition) {
		std::fprintf(stderr, "FAILED: %s\n", what);
		std::abort();
	}
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

	std::printf("all value-source checks passed\n");
	return 0;
}
