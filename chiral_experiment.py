#!/usr/bin/env python3
"""
Chiral Perturbation Experiment Script
Systematically tests different chiral_perturbation values and measures Xi diversity
"""
import subprocess
import re
import tempfile
import shutil
import os

# Test values as specified in the requirements
CHIRAL_VALUES = [0.0, 0.05, 0.1, 0.2, 0.3]
RESULTS_FILE = "chiral_experiment_results.txt"

def modify_research_file(chiral_value):
    """Modify research.rs to set the chiral_perturbation parameter"""
    research_path = "src/bin/research.rs"
    
    # Read the file
    with open(research_path, 'r') as f:
        content = f.read()
    
    # Replace the chiral_perturbation value
    pattern = r'chiral_perturbation: [\d.]+,'
    replacement = f'chiral_perturbation: {chiral_value},'
    
    new_content = re.sub(pattern, replacement, content)
    
    # Write back
    with open(research_path, 'w') as f:
        f.write(new_content)

def run_experiment(chiral_value):
    """Run the experiment with the given chiral perturbation value"""
    print(f"\n=== Running experiment with chiral_perturbation = {chiral_value} ===")
    
    # Modify the file
    modify_research_file(chiral_value)
    
    try:
        # Run the experiment
        result = subprocess.run(
            ["cargo", "run", "--release", "--bin", "research", "--", "--level", "3"],
            cwd=".",
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        
        if result.returncode != 0:
            print(f"Error running experiment: {result.stderr}")
            return None
        
        return result.stdout
    except subprocess.TimeoutExpired:
        print(f"Experiment timed out for chiral_perturbation = {chiral_value}")
        return None
    except Exception as e:
        print(f"Exception running experiment: {e}")
        return None

def parse_output(output):
    """Parse the experimental output to extract key metrics"""
    if not output:
        return {}
    
    metrics = {}
    lines = output.strip().split('\n')
    
    for line in lines:
        if ':' in line:
            key, value = line.split(':', 1)
            key = key.strip()
            value = value.strip()
            
            # Try to parse as float
            try:
                metrics[key] = float(value)
            except ValueError:
                metrics[key] = value
    
    return metrics

def main():
    """Run the full experiment suite"""
    print("Chiral Perturbation Experiment - Testing Xi Diversity Impact")
    print("=" * 60)
    
    results = []
    
    # Run experiments
    for chiral_value in CHIRAL_VALUES:
        output = run_experiment(chiral_value)
        if output:
            metrics = parse_output(output)
            if metrics:
                metrics['chiral_perturbation'] = chiral_value
                results.append(metrics)
                
                print(f"Results for chiral_perturbation = {chiral_value}:")
                print(f"  fitness: {metrics.get('fitness', 'N/A'):.6f}")
                print(f"  xi_diversity: {metrics.get('xi_diversity', 'N/A'):.4f}")
                print(f"  phase_coherence: {metrics.get('phase_coherence', 'N/A'):.4f}")
                print(f"  cluster_separation: {metrics.get('cluster_separation', 'N/A'):.4f}")
    
    # Write results to file
    with open(RESULTS_FILE, 'w') as f:
        f.write("Chiral Perturbation Experiment Results\n")
        f.write("=====================================\n\n")
        
        # Header
        f.write("chiral_perturbation,fitness,xi_diversity,phase_coherence,cluster_separation,consciousness,hall_quality,dream_efficiency\n")
        
        # Data rows
        for result in results:
            f.write(f"{result.get('chiral_perturbation', 0.0):.2f},")
            f.write(f"{result.get('fitness', 0.0):.6f},")
            f.write(f"{result.get('xi_diversity', 0.0):.4f},")
            f.write(f"{result.get('phase_coherence', 0.0):.4f},")
            f.write(f"{result.get('cluster_separation', 0.0):.4f},")
            f.write(f"{result.get('consciousness', 0.0):.4f},")
            f.write(f"{result.get('hall_quality', 0.0):.4f},")
            f.write(f"{result.get('dream_efficiency', 0.0):.4f}\n")
    
    # Summary
    print(f"\n{'='*60}")
    print("EXPERIMENT SUMMARY")
    print(f"{'='*60}")
    
    if results:
        print(f"{'Chiral':>6} | {'Fitness':>8} | {'Xi Div':>6} | {'Phase':>6} | {'Cluster':>7}")
        print("-" * 45)
        
        for result in results:
            print(f"{result.get('chiral_perturbation', 0.0):>6.2f} | "
                  f"{result.get('fitness', 0.0):>8.6f} | "
                  f"{result.get('xi_diversity', 0.0):>6.4f} | "
                  f"{result.get('phase_coherence', 0.0):>6.4f} | "
                  f"{result.get('cluster_separation', 0.0):>7.4f}")
        
        print(f"\nDetailed results saved to: {RESULTS_FILE}")
        
        # Find best Xi diversity
        best_xi = max(results, key=lambda r: r.get('xi_diversity', 0.0))
        best_fitness = min(results, key=lambda r: r.get('fitness', 999.0))
        
        print(f"\nBest Xi diversity: {best_xi.get('xi_diversity', 0.0):.4f} at chiral_perturbation = {best_xi.get('chiral_perturbation', 0.0):.2f}")
        print(f"Best fitness: {best_fitness.get('fitness', 999.0):.6f} at chiral_perturbation = {best_fitness.get('chiral_perturbation', 0.0):.2f}")
    else:
        print("No successful experiments completed.")

if __name__ == "__main__":
    main()