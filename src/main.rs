use std::{ fs, collections::HashMap };
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "C++ Project Generator", about = "This tool generates base C++ project.")]
struct Config {
    #[structopt(short, long, default_value = "project")]
    name: String,
    
    #[structopt(long, default_value = "exec")]
    config_type: String,

    #[structopt(long)]
    use_clang_tidy: bool,

    #[structopt(long)]
    use_conan: bool,

    #[structopt(short, long, default_value = "cmake")]
    generator: String,
}

static MAIN_CPP: &str = "\
#include <iostream>

int main() {
    std::cout << \"Hello World!\";\n
    return 0;
}";

static CLANG_TIDY: &str = "\
Checks: '*,-llvm*,-google*,-abseil*,-altera*,-android*,-boost*,-darwin*,-fuchsia*,-hicpp*,-linuxkernel*,-mpi*,-objc*,-openmp*,-zircon*,-modernize-use-trailing-return-type'
WarningsAsErrors: 'bugprone-use-after-move'
";

static GITIGNORE: &str = "\
/.vs
/Folder.DotSettings.user
/build
/proj_files
/.vscode
/.cache
**/CMakeCache.txt";

trait Generator {
    fn generate_build_file(&self, config: &Config);
    fn setup_cmd(&self) -> &'static str;
    fn build_cmd(&self) -> &'static str;
}

struct CMakeGen;

impl Generator for CMakeGen {
    fn generate_build_file(&self, config: &Config) {
        let mut cmake_file = String::new();
        cmake_file.push_str("cmake_minimum_required(VERSION 3.20)\n");
        cmake_file.push_str(format!("project({} VERSION 0.1.0)\n", config.name).as_str());

        if config.use_clang_tidy {
            cmake_file.push_str("set(CMAKE_CXX_CLANG_TIDY \"clang-tidy;-format-style=file;--use-color;-header-filter=.*\")\n");
        }

        cmake_file.push_str(format!("set({}_SRC_DIR \"src\")\n", config.name.to_uppercase()).as_str());
        cmake_file.push_str(format!("set({}_INCLUDE_DIR \"include\")\n", config.name.to_uppercase()).as_str());
        cmake_file.push_str(format!("include_directories({} PUBLIC {}_INCLUDE_DIR)\n", config.name, config.name.to_uppercase()).as_str());
        
        if config.use_conan {
            cmake_file.push_str("include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)\n");
        }

        if config.config_type == "exec" {
            cmake_file.push_str(format!("add_executable({} src/main.cpp)\n", config.name).as_str());
        } 
        else if config.config_type == "lib" {
            cmake_file.push_str(format!("add_library({0} src/{0}.cpp)\n", config.name).as_str());
        }

        if config.use_conan {
            cmake_file.push_str(format!("target_link_libraries({} ${{CONAN_LIBS}})\n", config.name).as_str());
        }

        cmake_file.push_str("set(ERROR_LIST \"-Werror=return-type -Werror=unused-result\")\n");
        cmake_file.push_str("set(CMAKE_CXX_FLAGS \"${CMAKE_CXX_FLAGS} -std=c++20 -Wall -Wextra ${ERROR_LIST}\")\n");
        
        fs::write(&(config.name.clone() + "\\CMakeLists.txt"), cmake_file).expect("Failed to create CMake file");
    }

    fn setup_cmd(&self) -> &'static str {
        "cmake -G Ninja -S . -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=1"
    }

    fn build_cmd(&self) -> &'static str {
        "cmake --build ./build"
    }
}

struct PremakeGen;

impl Generator for PremakeGen {
    fn generate_build_file(&self, _config: &Config) {
        
    }

    fn setup_cmd(&self) -> &'static str {
        "premake5 vs2022"
    }

    fn build_cmd(&self) -> &'static str {
        ""
    }
}

impl Config {
    fn create_project(&self) {
        println!("Creating C++ project...");
        self.create_default_dirs();

        let supported_generators: HashMap<String, Box<dyn Generator>> = HashMap::from(
            [
                ("cmake".to_string(), Box::new(CMakeGen{}) as Box<dyn Generator>),
                //("premake", Premake(PremakeGen{})),
            ]
        );

        let gen = match supported_generators.get(&self.generator) {
            Some(value) => {
                value
            },
            None => { panic!("Specified unsupported generator") }
        };

        gen.generate_build_file(self);
        self.create_cmd_shell_files(gen.build_cmd().to_string(), gen.setup_cmd().to_string());

        if self.use_conan {
            let conan_file = format!("[requires]\n\n[generators]\n{}", self.generator);
            fs::write(&(self.name.clone() + "\\conanfile.txt"), conan_file).expect("Failed to create conanfile.txt file");
        }

        if self.use_clang_tidy {
            fs::write(&(self.name.clone() + "\\.clang-tidy"), CLANG_TIDY).expect("Failed to create .clang-tidy file");
        }

        fs::write(&(self.name.clone() + "\\.gitignore"), GITIGNORE).expect("Failed to create .gitignore file");

        println!("Done.");
    }

    fn create_default_dirs(&self) {
        let dirs = vec![
            self.name.clone(),
            self.name.clone() + "\\src",
            self.name.clone() + "\\include",
            self.name.clone() + "\\build",
        ];

        for dir in &dirs {
            fs::create_dir(dir).expect(format!("Failed to create \"{}\" directory", dir).as_str());
        }

        fs::write(dirs[1].clone() + "\\main.cpp", MAIN_CPP).expect("Failed to create main.cpp file");
    }

    fn create_cmd_shell_files(&self, build: String, mut setup: String) {
        if self.use_conan {
            setup = format!("cd build\nconan install .. --build missing\ncd ..\n{}", setup);
        }

        fs::write(&(self.name.clone() + "\\setup.bat"), setup).expect("Failed to create setup.bat file");
        fs::write(&(self.name.clone() + "\\build.bat"), build).expect("Failed to create build.bat file");
    }
}

fn main() {
    let config = Config::from_args();
    config.create_project();
}
